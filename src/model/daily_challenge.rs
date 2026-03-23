use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use chrono::{DateTime, NaiveDate, Utc};

// ─── DB row models ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyChallenge {
    pub id: String,
    pub challenge_date: NaiveDate,
    pub soal_id: i32,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

impl<'c> FromRow<'c, MySqlRow> for DailyChallenge {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(DailyChallenge {
            id: row.get("id"),
            challenge_date: row.get("challenge_date"),
            soal_id: row.get("soal_id"),
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyChallengeAttempt {
    pub id: String,
    pub user_id: String,
    pub challenge_date: NaiveDate,
    pub soal_id: i32,
    pub selected_answer: i8,
    pub is_correct: bool,
    pub time_taken_ms: i32,
    pub score: i32,
    pub answered_at: DateTime<Utc>,
}

impl<'c> FromRow<'c, MySqlRow> for DailyChallengeAttempt {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(DailyChallengeAttempt {
            id: row.get("id"),
            user_id: row.get("user_id"),
            challenge_date: row.get("challenge_date"),
            soal_id: row.get("soal_id"),
            selected_answer: row.get("selected_answer"),
            is_correct: row.get("is_correct"),
            time_taken_ms: row.get("time_taken_ms"),
            score: row.get("score"),
            answered_at: row.get("answered_at"),
        })
    }
}

// ─── Request DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitChallengeRequest {
    pub selected_answer: i8,
    pub time_taken_ms: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetChallengeRequest {
    pub soal_id: i32,
    pub challenge_date: String, // "YYYY-MM-DD"
}

// ─── Response DTOs ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct SoalForChallenge {
    pub id: i32,
    pub soal: String,
    pub opt1: String,
    pub opt2: String,
    pub opt3: Option<String>,
    pub opt4: Option<String>,
    pub opt5: Option<String>,
    pub pelajaran: Option<String>,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAttemptSummary {
    pub selected_answer: i8,
    pub is_correct: bool,
    pub score: i32,
    pub time_taken_ms: i32,
    pub answered_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodayChallengeResponse {
    pub challenge_date: String,
    pub soal: SoalForChallenge,
    pub already_answered: bool,
    pub user_attempt: Option<UserAttemptSummary>,
    pub correct_answer: Option<i8>, // only revealed after answering
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitChallengeResponse {
    pub is_correct: bool,
    pub correct_answer: i8,
    pub score: i32,
    pub rank: i64,
    pub total_participants: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub user_id: String,
    pub display_name: String,
    pub score: i32,
    pub time_taken_ms: i32,
    pub answered_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyChallengeLeaderboardResponse {
    pub challenge_date: String,
    pub total_participants: i64,
    pub entries: Vec<LeaderboardEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserChallengeStats {
    pub total_attempted: i64,
    pub total_correct: i64,
    pub current_streak: i64,
    pub best_streak: i64,
    pub avg_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminChallengeResponse {
    pub id: String,
    pub challenge_date: String,
    pub soal_id: i32,
    pub created_by: String,
    pub created_at: String,
}
