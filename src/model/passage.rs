use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;

/// A shared reading passage used by one or more soal
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Passage {
    pub id: i32,
    pub content: String,
    pub title: Option<String>,
    pub source: Option<String>,
    pub language: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl<'c> FromRow<'c, MySqlRow> for Passage {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Passage {
            id: row.get("id"),
            content: row.get("content"),
            title: row.get("title"),
            source: row.get("source"),
            language: row.get("language"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

/// Request payload for creating or updating a passage
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePassageRequest {
    pub content: String,
    pub title: Option<String>,
    pub source: Option<String>,
    pub language: Option<String>,
}

/// Abbreviated passage for embedding in soal responses
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PassageSummary {
    pub id: i32,
    pub content: String,
    pub title: Option<String>,
}

/// Paginated passage list response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedPassagesResponse {
    pub passages: Vec<Passage>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

/// Passage detail including list of soal that reference it
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PassageDetail {
    pub passage: Passage,
    pub soal_ids: Vec<i32>,
    pub soal_count: i64,
}
