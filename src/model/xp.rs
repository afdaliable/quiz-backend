use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct XpBreakdownResponse {
    pub quiz_complete: i32,
    pub correct_answers: i32,
    pub score_bonus: i32,
    pub total: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XpAwardResultResponse {
    pub xp_awarded: i32,
    pub total_xp: i64,
    pub leveled_up: bool,
    pub new_level: i32,
    pub new_level_name: String,
    pub new_level_icon: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteSessionWithXpResponse {
    // quiz session fields (flattened)
    pub id: String,
    pub user_id: String,
    pub paket_soal_id: Option<i32>,
    pub kategori_soal: String,
    pub nama_paket_soal: String,
    pub session_type: String,
    pub question_ids: Option<Vec<i32>>,
    pub current_question: i32,
    pub answers: Vec<Option<i32>>,
    pub marked_questions: Vec<bool>,
    pub time_remaining: Option<i32>,
    pub total_time: Option<i32>,
    pub is_completed: bool,
    pub score: i32,
    pub correct_answers: i32,
    pub incorrect_answers: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    // XP data
    pub xp_breakdown: Option<XpBreakdownResponse>,
    pub xp_result: Option<XpAwardResultResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserXpResponse {
    pub total_xp: i64,
    pub current_level: i32,
    pub level_name: String,
    pub level_icon: String,
    pub progress: f64,
    pub xp_in_level: i64,
    pub xp_to_next_level: Option<i64>,
    pub global_rank: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XpHistoryEntry {
    pub id: String,
    pub amount: i32,
    pub source: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XpHistoryResponse {
    pub entries: Vec<XpHistoryEntry>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardXpEntry {
    pub rank: i64,
    pub display_name: String,
    pub total_xp: i64,
    pub current_level: i32,
    pub level_name: String,
    pub level_icon: String,
    pub is_current_user: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardXpResponse {
    pub entries: Vec<LeaderboardXpEntry>,
    pub current_user_rank: Option<i64>,
}
