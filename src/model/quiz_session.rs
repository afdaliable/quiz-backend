use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuizSession {
    pub id: String,
    pub user_id: String,
    pub paket_soal_id: Option<i32>,       // NULL untuk random session
    pub kategori_soal: String,
    pub nama_paket_soal: String,
    pub session_type: String,             // "standard" | "random"
    pub question_ids: Option<String>,     // JSON Vec<i32>, diisi untuk random session
    pub current_question: i32,
    pub answers: Option<String>,          // JSON string of Vec<Option<i32>>
    pub marked_questions: Option<String>, // JSON string of Vec<bool>
    pub time_remaining: Option<i32>,
    pub total_time: Option<i32>,
    pub is_completed: bool,
    pub score: i32,
    pub correct_answers: i32,
    pub incorrect_answers: i32,
    pub pomodoro_enabled: bool,
    pub pomodoro_sessions: i32,
    pub pomodoro_focus_minutes: i32,
    pub pomodoro_questions_answered: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl<'c> FromRow<'c, MySqlRow> for QuizSession {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(QuizSession {
            id: row.get("id"),
            user_id: row.get("user_id"),
            paket_soal_id: row.try_get("paket_soal_id").ok(),
            kategori_soal: row.get("kategori_soal"),
            nama_paket_soal: row.get("nama_paket_soal"),
            session_type: row.try_get("session_type").unwrap_or_else(|_| "standard".to_string()),
            question_ids: row.try_get("question_ids").ok(),
            current_question: row.get("current_question"),
            answers: row.get("answers"),
            marked_questions: row.get("marked_questions"),
            time_remaining: row.get("time_remaining"),
            total_time: row.get("total_time"),
            is_completed: row.get("is_completed"),
            score: row.get("score"),
            correct_answers: row.get("correct_answers"),
            incorrect_answers: row.get("incorrect_answers"),
            pomodoro_enabled: row.try_get("pomodoro_enabled").unwrap_or(false),
            pomodoro_sessions: row.try_get("pomodoro_sessions").unwrap_or(0),
            pomodoro_focus_minutes: row.try_get("pomodoro_focus_minutes").unwrap_or(0),
            pomodoro_questions_answered: row.try_get("pomodoro_questions_answered").unwrap_or(0),
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
pub struct StartRandomSessionRequest {
    pub count: u32,               // hanya 10 | 20 | 30
    pub category: Option<String>, // filter by kategori_soal, None = semua
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
    // Pomodoro stats (optional — only present when Pomodoro was active)
    pub pomodoro_enabled: Option<bool>,
    pub pomodoro_sessions: Option<i32>,
    pub pomodoro_focus_minutes: Option<i32>,
    pub pomodoro_questions_answered: Option<i32>,
}

/// Soal yang dikembalikan ke client untuk sesi random — tanpa correct_answer
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RandomSessionSoal {
    pub id: i32,
    pub soal: String,
    pub question_type: String,
    pub opt1: Option<String>,
    pub opt2: Option<String>,
    pub opt3: Option<String>,
    pub opt4: Option<String>,
    pub opt5: Option<String>,
    pub solution: Option<String>,
    pub modul: Option<String>,
    pub pelajaran: Option<String>,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartRandomSessionResponse {
    pub session_id: String,
    pub session_type: String,
    pub nama_paket_soal: String,
    pub kategori_soal: String,
    pub total_time: i32,
    pub total_questions: usize,
    pub questions: Vec<RandomSessionSoal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuizSessionResponse {
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
    pub pomodoro_enabled: bool,
    pub pomodoro_sessions: i32,
    pub pomodoro_focus_minutes: i32,
    pub pomodoro_questions_answered: i32,
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
    pub session_type: String,
    pub score: i32,
    pub correct: i32,
    pub wrong: i32,
    pub unanswered: i32,
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

        let question_ids: Option<Vec<i32>> = session.question_ids
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok());

        QuizSessionResponse {
            id: session.id,
            user_id: session.user_id,
            paket_soal_id: session.paket_soal_id,
            kategori_soal: session.kategori_soal,
            nama_paket_soal: session.nama_paket_soal,
            session_type: session.session_type,
            question_ids,
            current_question: session.current_question,
            answers,
            marked_questions,
            time_remaining: session.time_remaining,
            total_time: session.total_time,
            is_completed: session.is_completed,
            score: session.score,
            correct_answers: session.correct_answers,
            incorrect_answers: session.incorrect_answers,
            pomodoro_enabled: session.pomodoro_enabled,
            pomodoro_sessions: session.pomodoro_sessions,
            pomodoro_focus_minutes: session.pomodoro_focus_minutes,
            pomodoro_questions_answered: session.pomodoro_questions_answered,
            created_at: session.created_at,
            updated_at: session.updated_at,
        }
    }
}
