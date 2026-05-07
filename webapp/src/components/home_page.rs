use crate::{
    config::ENV,
    models::{Hit, SearchRequest, SearchResponse},
};
use chrono::{DateTime, Datelike, Local, NaiveDateTime, Utc};
use gloo_net::http::Request;
use gloo_timers::callback::Timeout;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, KeyboardEvent, MouseEvent, Url, wasm_bindgen::JsValue};
use yew::prelude::*;

fn page_prev_icon() -> Html {
    html! {
        <svg width="1em" height="1em" viewBox="0 0 16 16" fill="none"
             stroke="currentColor" stroke-width="1"
             stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M12 5L8 8L12 11 M8 5L4 8L8 11"/>
        </svg>
    }
}

fn page_next_icon() -> Html {
    html! {
        <svg width="1em" height="1em" viewBox="0 0 16 16" fill="none"
             stroke="currentColor" stroke-width="1"
             stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M4 5L8 8L4 11 M8 5L12 8L8 11"/>
        </svg>
    }
}

fn sep_icon() -> Html {
    html! {
        <svg width="4" height="4" viewBox="0 0 8 8" fill="currentColor" aria-hidden="true">
            <circle cx="4" cy="4" r="2.5"/>
        </svg>
    }
}

#[function_component(HomePage)]
pub fn home_page() -> Html {
    let input_ref = use_node_ref();
    let results = use_state(|| Vec::<Hit>::new());
    let total_hits = use_state(|| 0u32);
    let total_pages = use_state(|| 0u32);
    let query_str = use_state(|| String::new());
    let searched = use_state(|| false);
    let original_placeholder = "Search...".to_string();
    let current_placeholder = use_state(|| original_placeholder.clone());
    let placeholder_reset_timeout = use_state(|| Option::<Timeout>::None);
    let current_page = use_state(|| 1u32);
    let loading = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let request_serial = use_mut_ref(|| 0u32);

    let on_home_click = {
        let results = results.clone();
        let total_hits = total_hits.clone();
        let total_pages = total_pages.clone();
        let query_str = query_str.clone();
        let input_ref = input_ref.clone();
        let searched = searched.clone();
        let current_page = current_page.clone();
        let loading = loading.clone();
        let error = error.clone();
        let request_serial = request_serial.clone();
        let current_placeholder = current_placeholder.clone();
        let placeholder_reset_timeout = placeholder_reset_timeout.clone();
        let original_placeholder = original_placeholder.clone();

        Callback::from(move |_| {
            results.set(vec![]);
            total_hits.set(0);
            total_pages.set(0);
            query_str.set(String::new());
            error.set(None);
            *request_serial.borrow_mut() += 1;
            current_placeholder.set(original_placeholder.clone());
            placeholder_reset_timeout.set(None);

            if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                input.set_value("");
                let _ = input.focus();
            }

            searched.set(false);
            current_page.set(1);
            loading.set(false);

            if let Some(window) = web_sys::window() {
                window.scroll_to_with_x_and_y(0.0, 0.0);
                let _ = window.history().expect("history").replace_state_with_url(&JsValue::null(), "", Some("/"));
            }
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
        let error = error.clone();
        let request_serial = request_serial.clone();

        Callback::from(move |mut page_to_fetch: u32| {
            let current_query_value = if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                input.value()
            } else {
                String::new()
            };

            if current_query_value.trim().is_empty() {
                current_placeholder.set("Type something to search".to_string());
                placeholder_reset_timeout.set(None);
                let current_placeholder_clone = current_placeholder.clone();
                let original_placeholder_clone = original_placeholder.clone();
                let timeout = Timeout::new(3000, move || {
                    current_placeholder_clone.set(original_placeholder_clone);
                });
                placeholder_reset_timeout.set(Some(timeout));
                return;
            }

            if page_to_fetch < 1 { page_to_fetch = 1; }
            if *total_pages > 0 && page_to_fetch > *total_pages {
                page_to_fetch = *total_pages;
            }

            error.set(None);
            let serial = {
                let mut s = request_serial.borrow_mut();
                *s += 1;
                *s
            };

            loading.set(true);
            searched.set(true);
            query_str.set(current_query_value.clone());

            let results = results.clone();
            let total_hits = total_hits.clone();
            let total_pages = total_pages.clone();
            let loading = loading.clone();
            let current_page = current_page.clone();
            let q_for_async = current_query_value.clone();
            let error = error.clone();
            let request_serial = request_serial.clone();

            spawn_local(async move {
                let update_url_bar = |query: &str, page: u32| {
                    if let Some(window) = web_sys::window() {
                        if let Ok(href) = window.location().href() {
                            if let Ok(url) = Url::new(&href) {
                                url.search_params().set("q", query);
                                url.search_params().set("p", &page.to_string());
                                let _ = window.history().expect("history").replace_state_with_url(&JsValue::null(), "", Some(&url.href()));
                            }
                        }
                    }
                };

                let search_query = SearchRequest { q: q_for_async.clone(), page: page_to_fetch };
                let response = Request::post(&format!("{}/search", ENV.backend_url))
                    .json(&search_query).expect("fail").send().await;

                if *request_serial.borrow() != serial {
                    loading.set(false);
                    return;
                }

                if let Ok(res) = response {
                    if res.ok() {
                        if let Ok(mut data) = res.json::<SearchResponse>().await {
                            let mut final_page = page_to_fetch;

                            if data.total_pages > 0 && page_to_fetch > data.total_pages {
                                final_page = data.total_pages;
                                let retry_query = SearchRequest { q: q_for_async.clone(), page: final_page };
                                if let Ok(retry_res) = Request::post(&format!("{}/search", ENV.backend_url))
                                    .json(&retry_query).expect("fail").send().await {
                                    if *request_serial.borrow() != serial {
                                        loading.set(false);
                                        return;
                                    }
                                    if let Ok(retry_data) = retry_res.json::<SearchResponse>().await {
                                        data = retry_data;
                                    }
                                }
                            }

                            if let Some(window) = web_sys::window() {
                                window.scroll_to_with_x_and_y(0.0, 0.0);
                            }

                            update_url_bar(&q_for_async, final_page);
                            current_page.set(final_page);
                            total_hits.set(data.total_hits);
                            total_pages.set(data.total_pages);
                            results.set(data.hits);
                        } else {
                            results.set(vec![]);
                            total_hits.set(0);
                            total_pages.set(0);
                            error.set(Some("Got an unexpected response. Try again.".to_string()));
                        }
                    } else {
                        results.set(vec![]);
                        total_hits.set(0);
                        total_pages.set(0);
                        error.set(Some("Search failed. Try again.".to_string()));
                    }
                } else {
                    results.set(vec![]);
                    total_hits.set(0);
                    total_pages.set(0);
                    error.set(Some("Can't connect. Check your connection.".to_string()));
                }
                loading.set(false);
            });
        })
    };

    {
        let on_search = on_search.clone();
        let input_ref = input_ref.clone();
        use_effect_with((), move |_| {
            if let Some(window) = web_sys::window() {
                if let Ok(href) = window.location().href() {
                    if let Ok(url) = Url::new(&href) {
                        let q = url.search_params().get("q").unwrap_or_default();
                        let p = url.search_params().get("p")
                            .and_then(|v| v.parse::<u32>().ok())
                            .unwrap_or(1)
                            .max(1);

                        if !q.trim().is_empty() {
                            if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                                input.set_value(&q);
                            }
                            on_search.emit(p);
                        } else if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                            let _ = input.focus();
                        }
                    }
                }
            }
            || {}
        });
    }

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

    let results_label = if *total_hits == 1 {
        "Found 1 result".to_string()
    } else {
        format!("Found {} results", display_hits_str)
    };

    let current_year = chrono::Local::now().year();

    let status_msg = if *loading {
        "Searching...".to_string()
    } else if *searched {
        if *total_hits > 0 {
            results_label.clone()
        } else {
            "No results".to_string()
        }
    } else {
        String::new()
    };

    html! {
        <div class={classes!("container", extra_class)}>
            <header class="search-form">
                <div class="logo-wrapper">
                    <button type="button" class="logo" aria-label="TOPOS" onclick={on_home_click}>
                        {"T"}
                        <img src="favicons/favicon-96x96.png" alt="" aria-hidden="true" class="logo-icon" />
                        {"POS"}
                    </button>
                </div>
                <div class="search-bar">
                    <input
                        ref={input_ref}
                        type="text"
                        placeholder={(*current_placeholder).clone()}
                        id="search-input"
                        class="search-input"
                        aria-label="Search TROW"
                        onkeydown={on_key_down}
                        oninput={on_input_change}
                        disabled={*loading}
                    />
                    <button onclick={on_search.reform(|_event: MouseEvent| 1)} class="search-button" disabled={*loading}>{"SEARCH"}</button>
                </div>
                <div class="placeholder"></div>
            </header>

            <div class="sr-only" aria-live="polite" aria-atomic="true" role="status">{ status_msg }</div>

            <main class="main-content">
                <div class="results-list">
                    if let Some(err_msg) = (*error).clone() {
                        <div class="error-message" role="alert">{ err_msg }</div>
                    }
                    if *loading {
                        <div class="loading-spinner">
                            <div></div>
                        </div>
                    } else if *searched {
                        if *total_hits > 0 {
                            <div class="search-info">
                                <div class="total-hits">{ results_label.clone() }</div>

                                if *total_pages > 1 {
                                    <nav aria-label="Search results pagination" class="pagination">
                                        <button type="button" class="page-button" aria-label="Previous page" onclick={on_prev_page.clone()} disabled={!has_prev}>
                                            { page_prev_icon() }
                                        </button>
                                        <div class="page-num">{ format!("{} / {}", *current_page, *total_pages) }</div>
                                        <button type="button" class="page-button" aria-label="Next page" onclick={on_next_page.clone()} disabled={!has_next}>
                                            { page_next_icon() }
                                        </button>
                                    </nav>
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
                                                <a href={format!("https://trow.cc/board/showtopic={}&view=findpost&p={}", f.tid, f.pid)} target="_blank" rel="noopener noreferrer">
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
                                                <span class="sep" aria-hidden="true">
                                                    { sep_icon() }
                                                </span>
                                                <span class="date">{ local_post_date }</span>
                                            </div>
                                        </div>
                                    }
                                })
                            }

                            <div class="search-info">
                                <div class="total-hits">{ results_label.clone() }</div>

                                if *total_pages > 1 {
                                    <nav aria-label="Search results pagination" class="pagination">
                                        <button type="button" class="page-button" aria-label="Previous page" onclick={on_prev_page.clone()} disabled={!has_prev}>
                                            { page_prev_icon() }
                                        </button>
                                        <div class="page-num">{ format!("{} / {}", *current_page, *total_pages) }</div>
                                        <button type="button" class="page-button" aria-label="Next page" onclick={on_next_page.clone()} disabled={!has_next}>
                                            { page_next_icon() }
                                        </button>
                                    </nav>
                                }
                            </div>

                            </div>

                        } else {
                            <div class="search-info">
                                <div class="total-hits">{"No results"}</div>
                            </div>
                        }
                    }
                </div>
            </main>

            <footer class="footer">
                <div class="description">
                    <div class="intro">
                        <span>{"Full-text search for\u{a0}"}</span>
                        <a href="https://trow.cc" target="_blank" rel="noopener noreferrer">{"TROW"}</a>
                        <span class="sep" aria-hidden="true">{ sep_icon() }</span>
                        <span>{"Star on\u{a0}"}</span>
                        <a href="https://github.com/locene/topos" target="_blank" rel="noopener noreferrer">{"GitHub"}</a>
                    </div>
                    <div class="copyright">{ format!("© {} Locene", current_year) }</div>
                </div>
            </footer>
        </div>
    }
}
