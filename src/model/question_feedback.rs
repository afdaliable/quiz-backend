use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ── Rating ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "ENUM", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RatingKind {
    Helpful,
    Confusing,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitRatingRequest {
    pub rating: RatingKind,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RatingSummary {
    pub helpful_count:   i64,
    pub confusing_count: i64,
    pub user_rating:     Option<RatingKind>,
}

// ── Laporan ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "ENUM", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReportReason {
    WrongAnswer,
    UnclearExplanation,
    NotRelevant,
    Duplicate,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "ENUM", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ReportStatus {
    Pending,
    Reviewed,
    Resolved,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitReportRequest {
    pub reason: ReportReason,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReportWithQuestion {
    pub id:             String,
    pub question_id:    i32,
    pub question_text:  String,
    pub reporter_name:  String,
    pub reason:         ReportReason,
    pub detail:         Option<String>,
    pub status:         ReportStatus,
    pub admin_note:     Option<String>,
    pub created_at:     DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct QuestionReportStats {
    pub question_id:      i32,
    pub question_text:    String,
    pub pending_count:    i64,
    pub total_count:      i64,
    pub last_reported_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateReportStatusRequest {
    pub status:     ReportStatus,
    pub admin_note: Option<String>,
}
