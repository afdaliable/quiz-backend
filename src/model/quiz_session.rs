use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuizSession {
    pub id: String,
    pub user_id: String,
    pub paket_soal_id: i32,
    pub kategori_soal: String,
    pub nama_paket_soal: String,
    pub current_question: i32,
    pub answers: Option<String>, // JSON string of Vec<Option<i32>>
    pub marked_questions: Option<String>, // JSON string of Vec<bool>
    pub time_remaining: Option<i32>,
    pub total_time: Option<i32>,
    pub is_completed: bool,
    pub score: i32,
    pub correct_answers: i32,
    pub incorrect_answers: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl<'c> FromRow<'c, MySqlRow> for QuizSession {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(QuizSession {
            id: row.get("id"),
            user_id: row.get("user_id"),
            paket_soal_id: row.get("paket_soal_id"),
            kategori_soal: row.get("kategori_soal"),
            nama_paket_soal: row.get("nama_paket_soal"),
            current_question: row.get("current_question"),
            answers: row.get("answers"),
            marked_questions: row.get("marked_questions"),
            time_remaining: row.get("time_remaining"),
            total_time: row.get("total_time"),
            is_completed: row.get("is_completed"),
            score: row.get("score"),
            correct_answers: row.get("correct_answers"),
            incorrect_answers: row.get("incorrect_answers"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateQuizSessionRequest {
    pub paket_soal_id: i32,
    pub kategori_soal: String,
    pub nama_paket_soal: String,
    pub total_time: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateQuizSessionRequest {
    pub current_question: Option<i32>,
    pub answers: Option<Vec<Option<i32>>>,
    pub marked_questions: Option<Vec<bool>>,
    pub time_remaining: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteQuizSessionRequest {
    pub answers: Vec<Option<i32>>,
    pub time_remaining: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuizSessionResponse {
    pub id: String,
    pub user_id: String,
    pub paket_soal_id: i32,
    pub kategori_soal: String,
    pub nama_paket_soal: String,
    pub current_question: i32,
    pub answers: Vec<Option<i32>>,
    pub marked_questions: Vec<bool>,
    pub time_remaining: Option<i32>,
    pub total_time: Option<i32>,
    pub is_completed: bool,
    pub score: i32,
    pub correct_answers: i32,
    pub incorrect_answers: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub user_id: String,
    pub display_name: String,
    pub picture_url: Option<String>,
    pub best_score: i32,
    pub total_quizzes: i64,
    pub correct_answers: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardQuery {
    pub paket_soal_id: Option<i32>,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuizHistoryEntry {
    pub id: String,
    pub package_name: String,
    pub category: String,
    pub score: i32,
    pub correct: i32,
    pub wrong: i32,
    pub total: i32,
    pub duration_seconds: i32,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuizHistoryResponse {
    pub data: Vec<QuizHistoryEntry>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuizHistoryQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl From<QuizSession> for QuizSessionResponse {
    fn from(session: QuizSession) -> Self {
        let answers: Vec<Option<i32>> = session.answers
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default();

        let marked_questions: Vec<bool> = session.marked_questions
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default();

        QuizSessionResponse {
            id: session.id,
            user_id: session.user_id,
            paket_soal_id: session.paket_soal_id,
            kategori_soal: session.kategori_soal,
            nama_paket_soal: session.nama_paket_soal,
            current_question: session.current_question,
            answers,
            marked_questions,
            time_remaining: session.time_remaining,
            total_time: session.total_time,
            is_completed: session.is_completed,
            score: session.score,
            correct_answers: session.correct_answers,
            incorrect_answers: session.incorrect_answers,
            created_at: session.created_at,
            updated_at: session.updated_at,
        }
    }
}