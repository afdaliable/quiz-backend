// use super::Group;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

/// Represents a Soal (Question) entity
#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct Soal {
    /// Unique identifier for the soal
    pub id: i32,
    /// The question text
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
}

/// Request payload for creating a new soal
#[derive(Debug, Deserialize, Serialize, ToSchema)]
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
        })
    }
}
