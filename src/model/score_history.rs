use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScoreDataPoint {
    pub session_id: String,
    pub paket_soal_id: Option<i32>,
    pub package_name: String,
    pub category: String,
    pub score: i32,
    pub correct: i32,
    pub incorrect: i32,
    pub unanswered: i32,
    pub duration_seconds: i32,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScoreSummary {
    pub average: f64,
    pub highest: i32,
    pub lowest: i32,
    pub trend: f64,
    pub total_attempts: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScoreHistoryResponse {
    pub data_points: Vec<ScoreDataPoint>,
    pub summary: ScoreSummary,
}
