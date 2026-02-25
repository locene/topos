use crate::{
    config::ENV,
    models::{Hit, SearchRequest, SearchResponse},
};
use chrono::{DateTime, Datelike, Local, NaiveDateTime, Utc};
use gloo_net::http::Request;
use gloo_timers::callback::Timeout;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, KeyboardEvent, MouseEvent};
use yew::prelude::*;

#[function_component(SearchView)]
pub fn search_view() -> Html {
    let input_ref = use_node_ref();
    let results = use_state(|| Vec::<Hit>::new());
    let total_hits = use_state(|| 0u32);
    let total_pages = use_state(|| 0u32);
    let query_str = use_state(|| String::new());
    let searched = use_state(|| false);
    let original_placeholder = "Enter search query".to_string();
    let current_placeholder = use_state(|| original_placeholder.clone());
    let placeholder_reset_timeout = use_state(|| Option::<Timeout>::None);
    let current_page = use_state(|| 1u32);
    let loading = use_state(|| false);

    let on_home_click = {
        let results = results.clone();
        let total_hits = total_hits.clone();
        let total_pages = total_pages.clone();
        let query_str = query_str.clone();
        let input_ref = input_ref.clone();
        let searched = searched.clone();
        let current_page = current_page.clone();
        let loading = loading.clone();

        Callback::from(move |_| {
            results.set(vec![]);
            total_hits.set(0);
            total_pages.set(0);
            query_str.set(String::new());

            if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                input.set_value("");
            }

            searched.set(false);
            current_page.set(1);
            loading.set(false);
        })
    };

    let on_search = {
        let results = results.clone();
        let total_hits = total_hits.clone();
        let total_pages = total_pages.clone();
        let query_str = query_str.clone();
        let input_ref = input_ref.clone();
        let searched = searched.clone();
        let current_placeholder = current_placeholder.clone();
        let placeholder_reset_timeout = placeholder_reset_timeout.clone();
        let original_placeholder = original_placeholder.clone();
        let current_page = current_page.clone();
        let loading = loading.clone();

        Callback::from(move |page_to_fetch: u32| {
            let results = results.clone();
            let total_hits = total_hits.clone();
            let total_pages = total_pages.clone();
            let query_str = query_str.clone();
            let input_ref = input_ref.clone();
            let searched = searched.clone();
            let current_page = current_page.clone();
            let loading = loading.clone();

            let current_query_value = if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                input.value()
            } else {
                String::new()
            };

            if current_query_value.trim().is_empty() {
                current_placeholder.set("Please enter a search query".to_string());

                placeholder_reset_timeout.set(None);

                let current_placeholder_clone = current_placeholder.clone();
                let original_placeholder_clone = original_placeholder.clone();
                let timeout = Timeout::new(3000, move || {
                    current_placeholder_clone.set(original_placeholder_clone);
                });
                placeholder_reset_timeout.set(Some(timeout));

                return;
            }

            query_str.set(current_query_value.clone());
            current_page.set(page_to_fetch);
            searched.set(true);
            loading.set(true);

            spawn_local(async move {
                if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                    let query_value = input.value();

                    let search_query = SearchRequest {
                        q: query_value,
                        page: page_to_fetch,
                    };

                    let response = Request::post(&format!("{}/search", ENV.backend_url))
                        .json(&search_query)
                        .expect("Failed to serialize request")
                        .send()
                        .await;

                    match response {
                        Ok(res) => {
                            if let Ok(data) = res.json::<SearchResponse>().await {
                                total_hits.set(data.total_hits);
                                total_pages.set(data.total_pages);
                                results.set(data.hits);
                            }
                        }
                        Err(err) => {
                            gloo_console::log!("Fetch error:", err.to_string());
                        }
                    }

                    loading.set(false);
                }
            });
        })
    };

    let on_key_down = {
        let on_search = on_search.clone();
        let loading = loading.clone();
        let input_ref = input_ref.clone();

        Callback::from(move |keyboard_event: KeyboardEvent| {
            if keyboard_event.key() == "Enter" && !*loading {
                keyboard_event.prevent_default();
                on_search.emit(1);
            }

            if keyboard_event.key() == "Escape" {
                keyboard_event.prevent_default();

                if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                    input.set_value("");
                }
            }
        })
    };

    let on_input_change = {
        let current_placeholder = current_placeholder.clone();
        let placeholder_reset_timeout = placeholder_reset_timeout.clone();
        let original_placeholder = original_placeholder.clone();

        Callback::from(move |_e: InputEvent| {
            if *current_placeholder != original_placeholder {
                current_placeholder.set(original_placeholder.clone());
                placeholder_reset_timeout.set(None);
            }
        })
    };

    let on_prev_page = {
        let on_search = on_search.clone();
        let current_page = current_page.clone();
        let loading = loading.clone();

        Callback::from(move |_| {
            if !*loading {
                let prev_page = *current_page - 1;
                if prev_page >= 1 {
                    on_search.emit(prev_page);
                }
            }
        })
    };

    let on_next_page = {
        let on_search = on_search.clone();
        let current_page = current_page.clone();
        let max_pages = *total_pages;
        let loading = loading.clone();

        Callback::from(move |_| {
            if !*loading {
                let next_page = *current_page + 1;
                if next_page <= max_pages {
                    on_search.emit(next_page);
                }
            }
        })
    };

    let extra_class = if *searched { "searched" } else { "" };

    let display_hits_str = if *total_hits >= 1000 {
        "1000+".to_string()
    } else {
        total_hits.to_string()
    };

    let has_prev = *current_page > 1;
    let has_next = *current_page < *total_pages;

    let current_year = chrono::Local::now().year();

    html! {
        <div class={classes!("container", extra_class)}>
            <div class="search-form">
                <div class="logo" onclick={on_home_click}>{"TOPOS"}</div>
                <div class="search-bar">
                    <input
                        ref={input_ref}
                        type="text"
                        placeholder={(*current_placeholder).clone()}
                        id="search-input"
                        class="search-input"
                        onkeydown={on_key_down}
                        oninput={on_input_change}
                        disabled={*loading}
                    />
                    <button onclick={on_search.reform(|_event: MouseEvent| 1)} class="search-button" disabled={*loading}>{"SEARCH"}</button>
                </div>
                <div class="placeholder"></div>
            </div>

            <div class="main-content">
                <div class="results-list">
                    if *loading {
                        <div class="loading-spinner">
                            <div></div>
                        </div>
                    } else if *searched {
                        if *total_hits > 0 {
                            <div class="search-info">
                                <div class="total-hits">{ format!("Found {} results", display_hits_str) }</div>

                                if *total_pages > 1 {
                                    <div class="pagination">
                                        <div class="page-button" onclick={on_prev_page.clone()} disabled={!has_prev}>{"«"}</div>
                                        <div class="page-num">{ format!("{} / {}", *current_page, *total_pages) }</div>
                                        <div class="page-button" onclick={on_next_page.clone()} disabled={!has_next}>{"»"}</div>
                                    </div>
                                }
                            </div>

                            <div>

                            {
                                for results.iter().map(|item| {
                                    let f = &item.formatted;

                                    let local_post_date = {
                                        let parse_format = "%Y-%m-%d %H:%M:%S UTC";
                                        match NaiveDateTime::parse_from_str(&f.post_date, parse_format) {
                                            Ok(naive_dt) => {
                                                let datetime_utc = DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc);
                                                let datetime_local = datetime_utc.with_timezone(&Local);
                                                datetime_local.format("%Y-%m-%d %H:%M:%S").to_string()
                                            }
                                            Err(_) => {
                                                f.post_date.clone()
                                            }
                                        }
                                    };

                                    let title_html = {
                                        html! {
                                            <div class="result-title">
                                                <a href={format!("https://trow.cc/board/showtopic={}&view=findpost&p={}", f.tid, f.pid)} target="_blank">
                                                    { Html::from_html_unchecked(f.title.clone().into()) }
                                                </a>
                                            </div>
                                        }
                                    };

                                    let content_html = {
                                        html! {
                                            <div class="result-snippet">
                                                { Html::from_html_unchecked(f.post_rendered.clone().into()) }
                                            </div>
                                        }
                                    };

                                   let author_name_html = {
                                        html! {
                                            <span class="author">
                                                <a href={format!("https://trow.cc/board/showuser={}", f.author.id)}>
                                                    { Html::from_html_unchecked(f.author.name.clone().into()) }
                                                </a>
                                            </span>
                                        }
                                    };

                                    html! {
                                        <div class="result-item" key={f.pid.clone()}>
                                            { title_html }
                                            { content_html }
                                            <div class="result-meta">
                                                { author_name_html }
                                                <span class="sep">{" • "}</span>
                                                <span class="date">{ local_post_date }</span>
                                            </div>
                                        </div>
                                    }
                                })
                            }

                            <div class="search-info">
                                <div class="total-hits">{ format!("Found {} results", display_hits_str) }</div>

                                if *total_pages > 1 {
                                    <div class="pagination">
                                        <div class="page-button" onclick={on_prev_page.clone()} disabled={!has_prev}>{"«"}</div>
                                        <div class="page-num">{ format!("{} / {}", *current_page, *total_pages) }</div>
                                        <div class="page-button" onclick={on_next_page.clone()} disabled={!has_next}>{"»"}</div>
                                    </div>
                                }
                            </div>

                            </div>

                        } else {
                            <div class="search-info">
                                <div class="total-hits">{"No results found"}</div>
                            </div>
                        }
                    }
                </div>
            </div>

            <div class="footer">
                <div class="description">
                    <div class="intro">
                        <span>{ Html::from_html_unchecked("A convenient&nbsp;".into()) }</span>
                        <a href="https://trow.cc" target="_blank">{"TROW"}</a>
                        <span>{ Html::from_html_unchecked("&nbsp;site&nbsp;".into()) }</span>
                        <a href="https://github.com/locene/topos" target="_blank">{"search engine"}</a>
                        <span>{"."}</span>
                    </div>
                    <div class="copyright">{ format!("© {} Locene", current_year) }</div>
                </div>
            </div>
        </div>
    }
}
