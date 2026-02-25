use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub q: String,
    pub page: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub hits: Vec<Hit>,
    pub query: String,
    pub processing_time_ms: u32,
    pub hits_per_page: u32,
    pub page: u32,
    pub total_pages: u32,
    pub total_hits: u32,
    pub request_uid: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Hit {
    #[serde(rename = "_formatted")]
    pub formatted: FormattedContent,

    #[serde(rename = "_rankingScore")]
    pub ranking_score: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct FormattedContent {
    pub pid: String,
    pub author: Author,
    pub description: String,
    pub post_date: String,
    pub post_rendered: String,
    pub tid: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Author {
    pub id: String,
    pub name: String,
}
