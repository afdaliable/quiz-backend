use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PremiumQuizAccess {
    pub id: i32,
    pub paket_soal_id: i32,
    pub min_plan_id: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PremiumQuizAccessResponse {
    pub id: i32,
    pub paket_soal_id: i32,
    pub paket_soal_name: String,
    pub min_plan_id: i32,
    pub min_plan_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePremiumQuizAccessRequest {
    pub paket_soal_id: i32,
    pub min_plan_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePremiumQuizAccessRequest {
    pub min_plan_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuizAccessCheckResponse {
    pub has_access: bool,
    pub required_plan_id: Option<i32>,
    pub required_plan_name: Option<String>,
} 