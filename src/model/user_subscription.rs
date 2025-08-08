use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserSubscription {
    pub id: i32,
    pub user_id: String,
    pub plan_id: i32,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSubscriptionWithPlan {
    pub id: i32,
    pub user_id: String,
    pub plan_id: i32,
    pub plan_name: String,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub status: String,
    pub is_lifetime: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSubscriptionResponse {
    pub id: i32,
    pub user_id: String,
    pub plan_id: i32,
    pub plan_name: String,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub status: String,
    pub is_lifetime: bool,
    pub days_remaining: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserSubscriptionRequest {
    pub user_id: String,
    pub plan_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserSubscriptionRequest {
    pub status: Option<String>,
    pub end_date: Option<DateTime<Utc>>,
} 