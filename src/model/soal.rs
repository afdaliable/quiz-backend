// use super::Group;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

/// Represents a Soal (Question) entity
#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct Soal {
    /// Unique identifier for the soal
    pub id: i32,
    /// The question text
    pub soal: String,
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
}

/// Request payload for creating a new soal
#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
pub struct CreateSoalRequest {
    /// Question text
    pub soal: String,
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
}

impl<'c> FromRow<'c, MySqlRow> for Soal {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Soal {
            id: row.get(0),
            soal: row.get(1),
            opt1: row.get(2),
            opt2: row.get(3),
            opt3: row.get(4),
            opt4: row.get(5),
            opt5: row.get(6),
            correct_answer: row.get(7),
            solution: row.get(8),
            sumberfile: row.get(9),
            modul: row.get(10),
            pelajaran: row.get(11),
            tag: row.get(12),
        })
    }
}

/// Extended Soal model for admin operations with additional metadata
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AdminSoal {
    /// Unique identifier for the soal
    pub id: i32,
    /// The question text
    pub soal: String,
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
            soal: row.get("soal"),
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
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            usage_count: row.get("usage_count"),
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
    /// Question text
    pub soal: String,
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
