use crate::{config, utils};
use anyhow::{Context, Result, ensure};
use chrono::DateTime;
use reqwest::StatusCode;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};
use utils::resume_point::ResumePoint;
use utils::skip_topics::SkipTopics;

fn build_trow_client() -> Result<reqwest::Client, reqwest::Error> {
    let client = reqwest::Client::builder().build()?;

    Ok(client)
}

fn build_meilisearch_client() -> Result<reqwest::Client, reqwest::Error> {
    let config = config::get_app_config();

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::try_from(format!("Bearer {}", config.meilisearch_key)).unwrap(),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    Ok(client)
}

pub async fn delete_index() -> Result<()> {
    let config = config::get_app_config();

    let mut resume_point = ResumePoint::new()?;
    resume_point.delete()?;

    let skip_topics = SkipTopics::new()?;
    skip_topics.delete()?;

    let meilisearch_client = build_meilisearch_client()?;

    let _ = meilisearch_client
        .delete(format!("{}/indexes/posts", config.meilisearch_url))
        .send()
        .await;

    println!("Index deleted successfully");
    Ok(())
}

pub async fn initialize_index() -> Result<(), reqwest::Error> {
    let config = config::get_app_config();

    let meilisearch_client = build_meilisearch_client()?;

    let res_result = meilisearch_client
        .post(format!("{}/indexes", config.meilisearch_url))
        .json(&json!({
            "uid": "posts",
            "primaryKey": "pid"
        }))
        .send()
        .await;

    match res_result {
        Ok(res) => {
            let status = res.status();

            if !status.is_success() {
                let err_body = res.text().await?;
                warn!(
                    "Index creation might have failed or index already exists. Status: {}, Body: {}",
                    status, err_body
                );
            } else {
                info!(
                    "Index 'posts' created successfully or already exists. Status: {}",
                    status
                );
            }
        }
        Err(e) => {
            error!("Failed to send request to create index: {:?}", e);
            return Err(e);
        }
    }

    let settings = [
        ("sortable-attributes", json!(["post_date"])),
        (
            "ranking-rules",
            json!([
                "words",
                "sort",
                "typo",
                "proximity",
                "attribute",
                "exactness"
            ]),
        ),
        ("filterable-attributes", json!(["tid"])),
    ];

    for (path, data) in settings {
        let url = format!("{}/indexes/posts/settings/{}", config.meilisearch_url, path);

        let response = meilisearch_client.put(&url).json(&data).send().await?;

        if let Err(e) = response.error_for_status() {
            let status = e
                .status()
                .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            error!(
                "Failed to configure '{}': Status: {}, Error: {:?}",
                path, status, e
            );
            return Err(e.into());
        }

        info!("Successfully configured '{}'", path);
    }

    info!("Index 'posts' initialized successfully");
    Ok(())
}

pub async fn process_documents() -> Result<()> {
    let config = config::get_app_config();

    let trow_client = build_trow_client()?;
    let meilisearch_client = build_meilisearch_client()?;

    let topic_url = format!(
        "{}/board/topics?limit=1&pinned_on_top=0&order_by=start_date",
        config.trow_api_url
    );

    let mut max_topic_id = 0;

    'max_topic_id_retry_loop: loop {
        let response = match trow_client
            .get(&topic_url)
            .basic_auth(
                config.trow_client_id.clone(),
                Some(config.trow_client_secret.clone()),
            )
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                eprintln!("failed to read error body {topic_url}: {e}");

                break 'max_topic_id_retry_loop;
            }
        };

        if let Some(limit) = response.headers().get("x-ratelimit-limit") {
            match limit.to_str() {
                Ok(s) => println!("X-RateLimit-Limit: {}", s),
                Err(_) => {
                    eprintln!("Warning: x-ratelimit-limit header contained non-ASCII characters.")
                }
            }
        }

        let response_status = response.status();
        let response_text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                eprintln!(
                    "Failed to read error body for status {}: {}",
                    response_status, e
                );
                "Failed to read error body".to_string()
            }
        };

        if !response_status.is_success() {
            eprintln!(
                "Error fetching topic {}. Status: {}, Body: {}",
                topic_url, response_status, response_text
            );

            if response_status == StatusCode::TOO_MANY_REQUESTS {
                sleep(Duration::from_secs(60)).await;
            }
        } else {
            let topic_json: Value = serde_json::from_str(&response_text)
                .context("failed to parse the max_topic_id response body")?;

            max_topic_id = topic_json[0]["tid"].as_u64().with_context(|| {
                format!("missing or invalid 'tid' field in the max_topic_id response")
            })? as u32;

            break 'max_topic_id_retry_loop;
        }
    }

    println!("Starting document processing for 1 to {}...", max_topic_id);

    let mut resume_point = ResumePoint::new()?;
    let skip_topics = SkipTopics::new()?;

    'outer_retry_loop: loop {
        let mut needs_retry_sleep = false;
        let current_topic_id = resume_point.get_id().unwrap_or(1);

        for topic_num in current_topic_id..=max_topic_id {
            if skip_topics.contains(topic_num)? {
                println!("Skipping topic {}: Known to be invalid", topic_num);
                continue;
            }

            let mut posts_offset: u64 = 0;

            'topic_processing_loop: loop {
                let meilisearch_url =
                    format!("{}/indexes/posts/documents/delete", config.meilisearch_url);

                let _ = meilisearch_client
                    .post(&meilisearch_url)
                    .json(&json!([{
                        "filter": format!("tid = {}", topic_num),
                    }]))
                    .send()
                    .await
                    .with_context(|| format!("failed to send delete request for filter 'tid = {topic_num}' to {meilisearch_url}"));

                println!("Try to delete topic {} from documents", topic_num);

                let posts_limit = 100;

                let topic_url = format!(
                    "{}/board/topics/{}?limit={}&offset={}",
                    config.trow_api_url, topic_num, posts_limit, posts_offset
                );

                println!("Fetching topic: {}", topic_url);

                let trow_response = match trow_client
                    .get(&topic_url)
                    .basic_auth(
                        config.trow_client_id.clone(),
                        Some(config.trow_client_secret.clone()),
                    )
                    .send()
                    .await
                {
                    Ok(res) => res,
                    Err(e) => {
                        eprintln!("failed to read error body {topic_url}: {e}");

                        if let Err(e) = skip_topics.add(topic_num) {
                            eprintln!("Failed to write to skip file: {}", e);
                        }

                        break 'outer_retry_loop;
                    }
                };

                let response_status = trow_response.status();
                let response_text = match trow_response.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        eprintln!(
                            "Failed to read error body for status {}: {}",
                            response_status, e
                        );
                        "Failed to read error body".to_string()
                    }
                };

                if !response_status.is_success() {
                    eprintln!(
                        "Error fetching topic {}. Status: {}, Body: {}",
                        topic_url, response_status, response_text
                    );

                    if response_status == StatusCode::TOO_MANY_REQUESTS {
                        needs_retry_sleep = true;
                        resume_point.set_id(topic_num)?;
                    } else {
                        if let Err(e) = skip_topics.add(topic_num) {
                            eprintln!(
                                "Failed to write to skip file for topic {}: {}",
                                topic_num, e
                            );
                        }
                    }

                    break 'topic_processing_loop;
                } else {
                    let topic_json: Value = serde_json::from_str(&response_text)
                        .context("Failed to parse JSON from success response")?;

                    let tid = topic_json["tid"].as_u64().with_context(|| {
                        format!("missing or invalid 'tid' field in the topic {topic_num}")
                    })? as u32;
                    let title = topic_json["title"].as_str().with_context(|| {
                        format!("missing or invalid 'title' field in the topic {topic_num}")
                    })?;
                    let description = topic_json["description"].as_str().with_context(|| {
                        format!("missing or invalid 'description' field in the topic {topic_num}")
                    })?;

                    let posts_list = topic_json["posts_list"].as_array().with_context(|| {
                        format!("missing or invalid 'posts_list' field in the topic {topic_num}")
                    })?;

                    for (i, post) in posts_list.iter().enumerate() {
                        let pid = post["pid"].as_u64().with_context(|| {
                            format!(
                                "missing or invalid 'pid' field in the topic {topic_num}, post_list {i}"
                            )
                        })? as u32;

                        let post_date =  DateTime::from_timestamp(
                            post["post_date"].as_u64().with_context(|| {
                                format!(
                                    "missing or invalid 'post_date' field in the topic {topic_num}, post {pid}"
                                )
                            })? as i64,
                            0,
                        ).with_context(|| {
                            format!(
                                "missing or invalid 'post_date' field in DateTime::from_timestamp in the topic {topic_num}, post {pid}"
                            )
                        })?.to_string();

                        let author_id = post["author"]["id"].as_u64().with_context(|| {
                            format!(
                                "missing or invalid 'author.id' field in the topic {topic_num}, post {pid}"
                            )
                        })? as u32;

                        let author_name = post["author"]["name"].as_str().with_context(|| {
                            format!(
                                "missing or invalid 'author.name' field in the topic {topic_num}, post {pid}"
                            )
                        })?;

                        let mut post_rendered = post["post_rendered"].as_str().with_context(|| {
                            format!(
                                "missing or invalid 'post_rendered' field in the topic {topic_num}, post {pid}"
                            )
                        })?.to_string();
                        post_rendered = utils::text_cleaner::strip_and_decode_html(&post_rendered);

                        let document_to_send = json!([{
                            "tid": tid,
                            "title": title,
                            "description": description,
                            "pid": pid,
                            "post_date": post_date,
                            "author": {
                                "id": author_id,
                                "name": author_name
                            },
                            "post_rendered": post_rendered
                        }]);

                        let meilisearch_url =
                            format!("{}/indexes/posts/documents", config.meilisearch_url);

                        let meilisearch_response = meilisearch_client
                            .post(&meilisearch_url)
                            .json(&document_to_send)
                            .send()
                            .await
                            .with_context(|| format!("failed to sending topic {topic_num}, post {pid} to {meilisearch_url}"))?
                            .error_for_status()
                            .with_context(|| {
                                format!(
                                    "server returned an error status for sending topic {topic_num}, post {pid} to {meilisearch_url}"
                                )
                            })?;

                        ensure!(
                            meilisearch_response.status().is_success(),
                            "failed to sending topic {topic_num}, post {pid} to index, status: {}, body: {}",
                            meilisearch_response.status(),
                            meilisearch_response
                                .text()
                                .await
                                .unwrap_or_else(|_| "failed to read error body".to_string())
                        );
                    }

                    let posts = topic_json["posts"].as_u64().with_context(|| {
                        format!("missing or invalid 'posts' field in the topic {topic_num}")
                    })? as u64;

                    posts_offset += posts_limit;

                    if posts_offset < posts {
                        continue;
                    } else {
                        break 'topic_processing_loop;
                    }
                }
            }

            if needs_retry_sleep {
                sleep(Duration::from_secs(60)).await;
                needs_retry_sleep = false;
            }

            if topic_num == max_topic_id {
                resume_point.delete()?;
                break 'outer_retry_loop;
            }
        }
    }

    println!("Document processing finished");
    Ok(())
}
