// use super::Group;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

/// Represents a Soal (Question) entity
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Soal {
    /// Unique identifier for the soal
    pub id: i32,
    /// Optional reference to a shared reading passage
    pub passage_id: Option<i32>,
    /// The question text
    pub soal: String,
    /// Question type discriminator: "multiple_choice" | "true_false" | "fill_blank"
    pub question_type: String,
    /// First option
    pub opt1: Option<String>,
    /// Second option
    pub opt2: Option<String>,
    /// Third option
    pub opt3: Option<String>,
    /// Fourth option
    pub opt4: Option<String>,
    /// Fifth option
    pub opt5: Option<String>,
    /// The correct answer
    pub correct_answer: Option<String>,
    /// Solution explanation
    pub solution: Option<String>,
    /// Source file
    pub sumberfile: Option<String>,
    /// Module
    pub modul: Option<String>,
    /// Subject/Lesson
    pub pelajaran: Option<String>,
    /// Tag
    pub tag: Option<String>,
    // Taxonomy FK fields
    pub track_id: Option<String>,
    pub category_id: Option<String>,
    pub subcategory_id: Option<String>,
    pub topic_id: Option<String>,
    /// Estimated difficulty: "easy" | "medium" | "hard"
    pub difficulty_est: String,
    /// Calculated difficulty: "easy" | "medium" | "hard"
    pub difficulty_calc: Option<String>,
    /// Bloom's taxonomy level
    pub bloom_level: Option<String>,
    /// Question format: "pg" | "true_false" | "fill_blank" | "essay"
    pub format: String,
    /// Source reference
    pub source: Option<String>,
    /// Question status: "draft" | "active" | "archived"
    pub status: String,
    pub p_value: Option<f64>,
    pub avg_time_sec: Option<f64>,
    pub attempt_count: i32,
}

/// Request payload for creating a new soal
#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
pub struct CreateSoalRequest {
    /// Optional reference to a shared reading passage
    pub passage_id: Option<i32>,
    /// Question text
    pub soal: String,
    /// Question type: "multiple_choice" | "true_false" | "fill_blank" (default: "multiple_choice")
    pub question_type: Option<String>,
    /// First option
    pub opt1: String,
    /// Second option
    pub opt2: String,
    /// Third option
    pub opt3: String,
    /// Fourth option
    pub opt4: String,
    /// Fifth option
    pub opt5: String,
    /// The correct answer
    pub correct_answer: String,
    /// Solution explanation
    pub solution: String,
    /// Source file
    pub sumberfile: Option<String>,
    /// Module
    pub modul: Option<String>,
    /// Subject/Lesson
    pub pelajaran: Option<String>,
    /// Tag
    pub tag: Option<String>,
    // Taxonomy FK fields
    pub track_id: Option<String>,
    pub category_id: Option<String>,
    pub subcategory_id: Option<String>,
    pub topic_id: Option<String>,
    pub difficulty_est: Option<String>,
    pub difficulty_calc: Option<String>,
    pub bloom_level: Option<String>,
    pub format: Option<String>,
    pub source: Option<String>,
    pub status: Option<String>,
}

impl<'c> FromRow<'c, MySqlRow> for Soal {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Soal {
            id: row.get("id"),
            passage_id: row.try_get("passage_id").unwrap_or(None),
            soal: row.get("soal"),
            question_type: row.try_get("question_type").unwrap_or_else(|_| "multiple_choice".to_string()),
            opt1: row.get("opt1"),
            opt2: row.get("opt2"),
            opt3: row.get("opt3"),
            opt4: row.get("opt4"),
            opt5: row.get("opt5"),
            correct_answer: row.get("correct_answer"),
            solution: row.get("solution"),
            sumberfile: row.get("sumberfile"),
            modul: row.get("modul"),
            pelajaran: row.get("pelajaran"),
            tag: row.get("tag"),
            track_id: row.try_get("track_id").unwrap_or(None),
            category_id: row.try_get("category_id").unwrap_or(None),
            subcategory_id: row.try_get("subcategory_id").unwrap_or(None),
            topic_id: row.try_get("topic_id").unwrap_or(None),
            difficulty_est: row.try_get("difficulty_est").unwrap_or_else(|_| "medium".to_string()),
            difficulty_calc: row.try_get("difficulty_calc").unwrap_or(None),
            bloom_level: row.try_get("bloom_level").unwrap_or(None),
            format: row.try_get("format").unwrap_or_else(|_| "pg".to_string()),
            source: row.try_get("source").unwrap_or(None),
            status: row.try_get("status").unwrap_or_else(|_| "draft".to_string()),
            p_value: row.try_get("p_value").unwrap_or(None),
            avg_time_sec: row.try_get("avg_time_sec").unwrap_or(None),
            attempt_count: row.try_get("attempt_count").unwrap_or(0),
        })
    }
}

/// Extended Soal model for admin operations with additional metadata
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AdminSoal {
    /// Unique identifier for the soal
    pub id: i32,
    /// Optional reference to a shared reading passage
    pub passage_id: Option<i32>,
    /// The question text
    pub soal: String,
    /// Question type discriminator: "multiple_choice" | "true_false" | "fill_blank"
    pub question_type: String,
    /// First option
    pub opt1: Option<String>,
    /// Second option
    pub opt2: Option<String>,
    /// Third option
    pub opt3: Option<String>,
    /// Fourth option
    pub opt4: Option<String>,
    /// Fifth option
    pub opt5: Option<String>,
    /// The correct answer
    pub correct_answer: Option<String>,
    /// Solution explanation
    pub solution: Option<String>,
    /// Source file
    pub sumberfile: Option<String>,
    /// Module
    pub modul: Option<String>,
    /// Subject/Lesson
    pub pelajaran: Option<String>,
    /// Tag
    pub tag: Option<String>,
    // Taxonomy FK fields
    pub track_id: Option<String>,
    pub category_id: Option<String>,
    pub subcategory_id: Option<String>,
    pub topic_id: Option<String>,
    /// Estimated difficulty: "easy" | "medium" | "hard"
    pub difficulty_est: String,
    /// Calculated difficulty: "easy" | "medium" | "hard"
    pub difficulty_calc: Option<String>,
    /// Bloom's taxonomy level
    pub bloom_level: Option<String>,
    /// Question format: "pg" | "true_false" | "fill_blank" | "essay"
    pub format: String,
    /// Source reference
    pub source: Option<String>,
    /// Question status: "draft" | "active" | "archived"
    pub status: String,
    pub p_value: Option<f64>,
    pub avg_time_sec: Option<f64>,
    pub attempt_count: i32,
    /// Creation date
    pub created_at: Option<DateTime<Utc>>,
    /// Last update date
    pub updated_at: Option<DateTime<Utc>>,
    /// Number of times this question has been used
    pub usage_count: i64,
}

impl<'c> FromRow<'c, MySqlRow> for AdminSoal {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(AdminSoal {
            id: row.get("id"),
            passage_id: row.try_get("passage_id").unwrap_or(None),
            soal: row.get("soal"),
            question_type: row.try_get("question_type").unwrap_or_else(|_| "multiple_choice".to_string()),
            opt1: row.get("opt1"),
            opt2: row.get("opt2"),
            opt3: row.get("opt3"),
            opt4: row.get("opt4"),
            opt5: row.get("opt5"),
            correct_answer: row.get("correct_answer"),
            solution: row.get("solution"),
            sumberfile: row.get("sumberfile"),
            modul: row.get("modul"),
            pelajaran: row.get("pelajaran"),
            tag: row.get("tag"),
            track_id: row.try_get("track_id").unwrap_or(None),
            category_id: row.try_get("category_id").unwrap_or(None),
            subcategory_id: row.try_get("subcategory_id").unwrap_or(None),
            topic_id: row.try_get("topic_id").unwrap_or(None),
            difficulty_est: row.try_get("difficulty_est").unwrap_or_else(|_| "medium".to_string()),
            difficulty_calc: row.try_get("difficulty_calc").unwrap_or(None),
            bloom_level: row.try_get("bloom_level").unwrap_or(None),
            format: row.try_get("format").unwrap_or_else(|_| "pg".to_string()),
            source: row.try_get("source").unwrap_or(None),
            status: row.try_get("status").unwrap_or_else(|_| "draft".to_string()),
            p_value: row.try_get("p_value").unwrap_or(None),
            avg_time_sec: row.try_get("avg_time_sec").unwrap_or(None),
            attempt_count: row.try_get("attempt_count").unwrap_or(0),
            created_at: row.try_get("created_at").unwrap_or(None),
            updated_at: row.try_get("updated_at").unwrap_or(None),
            usage_count: row.try_get("usage_count").unwrap_or(0),
        })
    }
}

/// Request for question search/filtering
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QuestionSearchRequest {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub search: Option<String>,
    pub modul: Option<String>,
    pub pelajaran: Option<String>,
    pub tag: Option<String>,
}

/// Paginated questions response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedQuestionsResponse {
    pub questions: Vec<AdminSoal>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

/// Request for updating a question
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateSoalRequest {
    /// Optional reference to a shared reading passage (null = unassign)
    pub passage_id: Option<i32>,
    /// Question text
    pub soal: String,
    /// Question type: "multiple_choice" | "true_false" | "fill_blank" (default: "multiple_choice")
    pub question_type: Option<String>,
    /// First option
    pub opt1: String,
    /// Second option
    pub opt2: String,
    /// Third option
    pub opt3: String,
    /// Fourth option
    pub opt4: String,
    /// Fifth option
    pub opt5: String,
    /// The correct answer
    pub correct_answer: String,
    /// Solution explanation
    pub solution: String,
    /// Source file
    pub sumberfile: Option<String>,
    /// Module
    pub modul: Option<String>,
    /// Subject/Lesson
    pub pelajaran: Option<String>,
    /// Tag
    pub tag: Option<String>,
    // Taxonomy FK fields
    pub track_id: Option<String>,
    pub category_id: Option<String>,
    pub subcategory_id: Option<String>,
    pub topic_id: Option<String>,
    pub difficulty_est: Option<String>,
    pub difficulty_calc: Option<String>,
    pub bloom_level: Option<String>,
    pub format: Option<String>,
    pub source: Option<String>,
    pub status: Option<String>,
}

/// Request for bulk importing questions
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct BulkImportRequest {
    pub questions: Vec<CreateSoalRequest>,
}

/// Response for bulk import
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkImportResponse {
    pub success_count: i32,
    pub failed_count: i32,
    pub errors: Vec<String>,
}

/// Enhanced CSV import request with validation options
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CsvImportRequest {
    /// Whether to skip invalid rows or fail completely
    pub skip_invalid_rows: bool,
    /// Maximum number of errors to allow before stopping
    pub max_errors: Option<usize>,
    /// Whether to validate only or actually import
    pub validate_only: bool,
}

/// CSV import response with detailed results
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CsvImportResponse {
    pub success_count: i32,
    pub failed_count: i32,
    pub skipped_count: i32,
    pub total_processed: i32,
    pub errors: Vec<CsvImportError>,
    pub warnings: Vec<CsvImportError>,
    pub import_id: Option<String>,
    pub estimated_time_seconds: u32,
}

/// Detailed error information for CSV imports
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct CsvImportError {
    pub row_number: usize,
    pub field: String,
    pub error_type: String,
    pub message: String,
    pub suggested_fix: Option<String>,
    pub raw_value: Option<String>,
}

/// Soal with its associated passage inlined (for quiz session responses)
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct SoalWithPassage {
    #[serde(flatten)]
    pub soal: Soal,
    pub passage: Option<crate::model::passage::PassageSummary>,
}
