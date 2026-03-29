use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ─────────────────────────────────────────────────────────────────────────────
// DB model structs (sqlx::FromRow)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AiUsageLog {
    pub id: i64,
    pub question_id: i64,
    pub job_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AiGeneratedContent {
    pub id: i64,
    pub question_id: i64,
    pub job_id: Option<String>,
    pub field_name: String,
    pub original_value: Option<String>,
    pub generated_value: String,
    pub provider: String,
    pub model: String,
    pub accepted: bool,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<String>,
    pub rejected: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AiBulkJob {
    pub id: String,
    pub status: String,
    pub total: i32,
    pub processed: i32,
    pub succeeded: i32,
    pub failed: i32,
    pub auto_save: bool,
    pub rate_limit_per_minute: i32,
    pub error_message: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Cost estimation
// ─────────────────────────────────────────────────────────────────────────────

/// Estimates USD cost based on token usage.
/// Rates as of 2026-03:
/// - DeepSeek Chat:    $0.07/1M input, $0.28/1M output
/// - Gemini 2.0 Flash: $0.075/1M input, $0.30/1M output
pub fn estimate_cost(provider: &str, prompt_tokens: i32, completion_tokens: i32) -> f64 {
    match provider {
        "deepseek" => {
            (prompt_tokens as f64 * 0.07 + completion_tokens as f64 * 0.28) / 1_000_000.0
        }
        "gemini" => {
            (prompt_tokens as f64 * 0.075 + completion_tokens as f64 * 0.30) / 1_000_000.0
        }
        _ => 0.0,
    }
}
