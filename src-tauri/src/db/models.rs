use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub avatar_local_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub username: Option<String>,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub avatar_local_path: Option<String>,
    pub post_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileState {
    pub profiles: Vec<Profile>,
    pub active_profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub id: String,
    pub kind: String,
    pub remote_url: Option<String>,
    pub preview_url: Option<String>,
    pub local_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub id: String,
    pub text: String,
    pub canonical_url: Option<String>,
    pub author_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tweet {
    pub id: String,
    pub text: String,
    pub archive_text: Option<String>,
    pub created_at: Option<String>,
    pub liked_at: Option<String>,
    pub canonical_url: String,
    pub enrichment_status: String,
    pub enrichment_error: Option<String>,
    pub author: Option<Author>,
    pub media: Vec<Media>,
    pub quote: Option<Quote>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TweetQuery {
    pub search: Option<String>,
    pub author: Option<String>,
    pub media_type: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    50
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TweetPage {
    pub items: Vec<Tweet>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub imported: u64,
    pub existing: u64,
    pub pending: u64,
    pub failed: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatus {
    pub phase: String,
    pub processed: u64,
    pub total: u64,
    pub failed: u64,
    pub message: String,
}

impl Default for JobStatus {
    fn default() -> Self {
        Self {
            phase: "idle".into(),
            processed: 0,
            total: 0,
            failed: 0,
            message: "Ready".into(),
        }
    }
}
