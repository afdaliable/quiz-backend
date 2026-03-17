use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};

/// Response untuk POST bookmark
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BookmarkResponse {
    pub success: bool,
    pub message: String,
    pub bookmark_count: i32,
}

/// Response untuk DELETE bookmark (single)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteBookmarkResponse {
    pub success: bool,
    pub deleted_question_id: i32,
    pub bookmark_count: i32,
}

/// Response untuk DELETE bookmark (bulk)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkDeleteResponse {
    pub success: bool,
    pub deleted_count: u64,
    pub bookmark_count: i32,
}

/// Request body untuk bulk delete
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkDeleteRequest {
    pub question_ids: Vec<i32>,
}

/// Request payload untuk bookmark
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BookmarkRequest {
    pub question_id: i32,
}

/// Count per kategori untuk badge filter
#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

/// Response list bookmark dengan pagination & kategori
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BookmarkListResponse {
    pub bookmarks: Vec<BookmarkedQuestion>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
    pub categories: Vec<CategoryCount>,
}

/// Detail soal yang di-bookmark
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BookmarkedQuestion {
    pub id: String,
    pub question_id: i32,
    pub soal: String,
    pub opt1: Option<String>,
    pub opt2: Option<String>,
    pub opt3: Option<String>,
    pub opt4: Option<String>,
    pub opt5: Option<String>,
    pub correct_answer: Option<String>,
    pub solution: Option<String>,
    pub modul: Option<String>,
    pub pelajaran: Option<String>,
    pub tag: Option<String>,
    pub question_type: Option<String>,
    pub bookmarked_at: Option<DateTime<Utc>>,
    pub quiz_name: String,
    pub question_number: i64,
}

impl<'c> FromRow<'c, MySqlRow> for BookmarkedQuestion {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(BookmarkedQuestion {
            id: row.get("id"),
            question_id: row.get("question_id"),
            soal: row.get("soal"),
            opt1: row.get("opt1"),
            opt2: row.get("opt2"),
            opt3: row.get("opt3"),
            opt4: row.get("opt4"),
            opt5: row.get("opt5"),
            correct_answer: row.get("correct_answer"),
            solution: row.get("solution"),
            modul: row.get("modul"),
            pelajaran: row.get("pelajaran"),
            tag: row.get("tag"),
            question_type: row.try_get("question_type").ok(),
            bookmarked_at: row.try_get("bookmarked_at").ok(),
            quiz_name: row.try_get("quiz_name").unwrap_or_default(),
            question_number: row.try_get("question_number").unwrap_or(0),
        })
    }
}
