use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Admin-curated reference text scoped to a subcategory (required) and
/// optionally a narrower topic -- used to ground AI enrich instead of
/// letting it answer from general knowledge alone.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MateriLibrary {
    pub id: String,
    pub subcategory_id: String,
    pub topic_id: Option<String>,
    pub title: String,
    pub content: String,
    pub source: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMateriLibraryRequest {
    pub subcategory_id: String,
    pub topic_id: Option<String>,
    pub title: String,
    pub content: String,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMateriLibraryRequest {
    pub subcategory_id: String,
    pub topic_id: Option<String>,
    pub title: String,
    pub content: String,
    pub source: Option<String>,
}
