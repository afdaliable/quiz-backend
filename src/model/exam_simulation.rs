use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExamSimulation {
    pub id: i32,
    pub nama_simulasi: String,
    pub deskripsi: Option<String>,
    pub paket_soal_id: Option<i32>,
    pub paket_soal_nama: Option<String>,
    pub generation_mode: String,
    pub exam_type: Option<String>,
    pub generation_config: Option<JsonValue>,
    pub navigation_mode: String,
    pub sections_json: Option<JsonValue>,
    pub duration_minutes: i32,
    pub total_questions: i32,
    pub passing_score: i32,
    pub is_premium: bool,
    pub max_attempts: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Optional aggregate stats (filled by admin list query)
    pub total_attempts: Option<i64>,
    pub avg_score: Option<f64>,
    pub pass_rate: Option<f64>,
    // User-specific (filled by user-facing list)
    pub user_attempts: Option<i64>,
    pub best_score: Option<i32>,
    pub can_attempt: Option<bool>,
}

impl<'c> FromRow<'c, MySqlRow> for ExamSimulation {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(ExamSimulation {
            id: row.get("id"),
            nama_simulasi: row.get("nama_simulasi"),
            deskripsi: row.try_get("deskripsi").ok(),
            paket_soal_id: row.try_get("paket_soal_id").ok(),
            paket_soal_nama: row.try_get("paket_soal_nama").ok(),
            generation_mode: row.get("generation_mode"),
            exam_type: row.try_get::<Option<String>, _>("exam_type").ok().flatten(),
            generation_config: row.try_get("generation_config").ok(),
            navigation_mode: row.try_get("navigation_mode").unwrap_or_else(|_| "free".to_string()),
            sections_json: row
                .try_get::<Option<String>, _>("sections_json")
                .ok()
                .flatten()
                .and_then(|s| serde_json::from_str(&s).ok()),
            duration_minutes: row.get("duration_minutes"),
            total_questions: row.get("total_questions"),
            passing_score: row.get("passing_score"),
            is_premium: row.get("is_premium"),
            max_attempts: row.get("max_attempts"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            total_attempts: row.try_get("total_attempts").ok(),
            avg_score: row.try_get("avg_score").ok(),
            pass_rate: row.try_get("pass_rate").ok(),
            user_attempts: row.try_get("user_attempts").ok(),
            best_score: row.try_get("best_score").ok(),
            can_attempt: row.try_get("can_attempt").ok(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateExamSimulationRequest {
    pub nama_simulasi: String,
    pub deskripsi: Option<String>,
    pub paket_soal_id: Option<i32>,
    pub generation_mode: String,
    pub generation_config: Option<JsonValue>,
    pub duration_minutes: i32,
    pub total_questions: i32,
    pub passing_score: i32,
    #[serde(default)]
    pub is_premium: bool,
    #[serde(default)]
    pub max_attempts: i32,
    /// Which product lists this simulasi, e.g. "skd", "lpdp", "upkp". Apps
    /// filter GET /simulasi-ujian on it, so a simulasi without one shows up
    /// in none of them.
    #[serde(default)]
    pub exam_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UpdateExamSimulationRequest {
    pub nama_simulasi: Option<String>,
    pub deskripsi: Option<String>,
    pub paket_soal_id: Option<i32>,
    pub generation_mode: Option<String>,
    pub generation_config: Option<JsonValue>,
    pub duration_minutes: Option<i32>,
    pub total_questions: Option<i32>,
    pub passing_score: Option<i32>,
    pub is_premium: Option<bool>,
    pub max_attempts: Option<i32>,
    pub is_active: Option<bool>,
    pub exam_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimulasiUserAttempt {
    pub id: i32,
    pub simulasi_id: i32,
    pub user_id: String,
    pub quiz_session_id: String,
    pub attempt_number: i32,
    pub score: Option<i32>,
    pub is_passed: Option<bool>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    // joined fields (admin-facing)
    pub user_email: Option<String>,
    pub user_display_name: Option<String>,
}

impl<'c> FromRow<'c, MySqlRow> for SimulasiUserAttempt {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(SimulasiUserAttempt {
            id: row.get("id"),
            simulasi_id: row.get("simulasi_id"),
            user_id: row.get("user_id"),
            quiz_session_id: row.get("quiz_session_id"),
            attempt_number: row.get("attempt_number"),
            score: row.try_get("score").ok(),
            is_passed: row.try_get("is_passed").ok(),
            completed_at: row.try_get("completed_at").ok(),
            created_at: row.get("created_at"),
            user_email: row.try_get("user_email").ok(),
            user_display_name: row.try_get("user_display_name").ok(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartSimulasiResponse {
    pub session_id: String,
    pub simulasi_id: i32,
    pub attempt_number: i32,
    pub duration_minutes: i32,
    pub total_questions: i32,
    pub passing_score: i32,
    pub questions: Vec<JsonValue>,
}
