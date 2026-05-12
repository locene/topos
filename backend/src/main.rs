use axum::{
    Json, Router,
    extract::Request,
    http::{StatusCode, header},
    middleware::{Next, from_fn},
    response::{IntoResponse, Response},
    routing::post,
};
use chrono::Utc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::cors::CorsLayer;
use tracing::{Level, info};

mod config;
mod sync_posts;
mod utils {
    pub mod resume_point;
    pub mod skip_topics;
    pub mod text_cleaner;
}

#[derive(Deserialize)]
struct ClientRequest {
    q: String,
    page: u32,
}

#[derive(Serialize)]
struct MeiliSearchRequest {
    q: String,
    #[serde(rename = "hitsPerPage")]
    hits_per_page: u32,
    page: u32,
    #[serde(rename = "attributesToSearchOn")]
    attributes_to_search_on: Vec<String>,
    #[serde(rename = "attributesToRetrieve")]
    attributes_to_retrieve: Vec<String>,
    #[serde(rename = "attributesToCrop")]
    attributes_to_crop: Vec<String>,
    #[serde(rename = "cropLength")]
    crop_length: u32,
    #[serde(rename = "attributesToHighlight")]
    attributes_to_highlight: Vec<String>,
    #[serde(rename = "showRankingScore")]
    show_ranking_score: bool,
    #[serde(rename = "sort")]
    sort: Vec<String>,
}

#[tokio::main]
async fn main() {
    let config = config::get_app_config();

    let max_level = Level::from_str(&config.log_level).unwrap_or(Level::INFO);
    tracing_subscriber::fmt().with_max_level(max_level).init();

    info!("Application starting...");
    info!("MeiliSearch URL: {}", config.meilisearch_url);
    info!("Configured log level: {:?}", max_level);

    start_daily_cron_scheduler().await;

    let admin_routes = Router::new()
        .route("/posts/delete", post(posts_delete))
        .route("/posts/initialize", post(posts_initialize))
        .route("/posts/sync-on-demand", post(posts_sync_on_demand))
        .layer(from_fn(admin_auth));

    let app = Router::new()
        .route("/search", post(search))
        .nest("/admin", admin_routes)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let _ = axum::serve(listener, app).await;
}

async fn admin_auth(req: Request, next: Next) -> Response {
    let config = config::get_app_config();

    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let expected_token = format!("Bearer {}", config.admin_token);

    if let Some(auth_header) = auth_header {
        if auth_header == expected_token {
            return next.run(req).await;
        }
    }

    StatusCode::UNAUTHORIZED.into_response()
}

async fn search(Json(payload): Json<ClientRequest>) -> Json<serde_json::Value> {
    let config = config::get_app_config();

    let processed_q = payload
        .q
        .split_whitespace()
        .map(|word| format!("\"{}\"", word))
        .collect::<Vec<String>>()
        .join(" ");

    let meili_payload = MeiliSearchRequest {
        q: processed_q,
        hits_per_page: 10,
        page: payload.page,
        attributes_to_search_on: vec![
            "title".to_string(),
            "description".to_string(),
            "post_rendered".to_string(),
        ],
        attributes_to_retrieve: vec!["_formatted".to_string()],
        attributes_to_crop: vec!["post_rendered".to_string()],
        crop_length: 100,
        attributes_to_highlight: vec!["*".to_string()],
        show_ranking_score: true,
        sort: vec!["post_date:desc".to_string()],
    };

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/indexes/posts/search", config.meilisearch_url))
        .header(
            "Authorization",
            format!("Bearer {}", config.meilisearch_key),
        )
        .json(&meili_payload)
        .send()
        .await;

    match response {
        Ok(res) => {
            let body = res
                .json::<serde_json::Value>()
                .await
                .unwrap_or(serde_json::json!({"error": "Failed to parse meili response"}));
            Json(body)
        }
        Err(e) => {
            Json(serde_json::json!({ "error": format!("Request to Meilisearch failed: {}", e) }))
        }
    }
}

async fn posts_delete() {
    if let Err(e) = sync_posts::delete_index().await {
        tracing::error!("Failed to delete index: {:?}", e);
    }
}

async fn posts_initialize() {
    if let Err(e) = sync_posts::initialize_index().await {
        tracing::error!("Failed to initialize index: {:?}", e);
    }
}

static IS_SYNC_RUNNING: Lazy<Arc<Mutex<bool>>> = Lazy::new(|| Arc::new(Mutex::new(false)));

async fn run_sync_task_with_mutex_management(task_origin: &str) {
    let mut is_sync_running_guard = IS_SYNC_RUNNING.lock().await;

    if *is_sync_running_guard {
        info!("[{}] Task already running. Skipping.", task_origin);
        return;
    }

    *is_sync_running_guard = true;
    info!("[{}] Task started. Status set to Running.", task_origin);
    drop(is_sync_running_guard);

    info!("Starting document processing...");
    if let Err(e) = sync_posts::process_documents().await {
        tracing::error!("Failed to process documents: {:?}", e);
    }
    info!("Document processing finished.");

    let mut sync_finished_guard = IS_SYNC_RUNNING.lock().await;
    *sync_finished_guard = false;
    info!(
        "[STATUS] [{}] Task finished. Status set to Idle.",
        task_origin
    );
}

async fn start_daily_cron_scheduler() {
    let sched = JobScheduler::new()
        .await
        .expect("Failed to create scheduler");

    sched.start().await.expect("Failed to start scheduler");

    let job = Job::new_async("0 0 18 * * *", move |_uuid, _l| {
        Box::pin(async move {
            info!("[{}] Cron job triggered.", Utc::now().to_rfc3339());
            run_sync_task_with_mutex_management("Cron Scheduler").await;
        })
    })
    .expect("Failed to create cron job");

    sched.add(job).await.expect("Failed to add cron job");

    info!("Cron scheduler started. Daily sync task scheduled for next time.");
}

async fn posts_sync_on_demand() -> Response {
    let is_running = *IS_SYNC_RUNNING.lock().await;

    if is_running {
        info!("On-demand sync: Task already running. Skipping trigger.");
        return (StatusCode::OK, "Task already running. Please wait.").into_response();
    }

    tokio::spawn(async move {
        run_sync_task_with_mutex_management("On-Demand API").await;
    });

    (StatusCode::ACCEPTED, "Sync task initiated in background.").into_response()
}
