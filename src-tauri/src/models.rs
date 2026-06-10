use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEntry {
    pub id: String,
    pub content_type: String,
    pub content: String,
    pub source_app: String,
    pub preview: String,
    pub created_at: String,
    pub image_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardFilter {
    pub query: Option<String>,
    pub content_type: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}
