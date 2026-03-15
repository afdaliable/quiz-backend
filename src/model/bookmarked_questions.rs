use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};

/// Response when bookmarking/unbookmarking a question
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BookmarkResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Message describing the result
    pub message: String,
    /// Total number of bookmarks for this user
    pub bookmark_count: i32,
}

/// Request payload for bookmarking a question
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BookmarkRequest {
    /// ID of the question to bookmark
    pub question_id: i32,
}

/// Bookmarked question details
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BookmarkedQuestion {
    /// Bookmark ID
    pub id: String,
    /// Question ID
    pub question_id: i32,
    /// Question text
    pub soal: String,
    /// Option 1
    pub opt1: Option<String>,
    /// Option 2
    pub opt2: Option<String>,
    /// Option 3
    pub opt3: Option<String>,
    /// Option 4
    pub opt4: Option<String>,
    /// Option 5
    pub opt5: Option<String>,
    /// Correct answer
    pub correct_answer: Option<String>,
    /// Explanation/solution
    pub solution: Option<String>,
    /// Module
    pub modul: Option<String>,
    /// Subject (pelajaran)
    pub pelajaran: Option<String>,
    /// Tag
    pub tag: Option<String>,
    /// Question type
    pub question_type: Option<String>,
    /// Created at timestamp
    pub created_at: String,
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
            created_at: row.get("created_at"),
        })
    }
}
