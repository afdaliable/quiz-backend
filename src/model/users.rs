use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use chrono::{DateTime, NaiveDate, Utc};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub username: Option<String>,
    pub picture_url: Option<String>,
    pub phone_number: Option<String>,
    pub profile_public: bool,
    pub privacy_settings: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub onboarding_completed: bool,
    pub onboarding_goals: Option<String>,
    pub exam_timeframe: Option<String>,
    pub target_exam_date: Option<NaiveDate>,
}

impl<'c> FromRow<'c, MySqlRow> for User {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(User {
            id: row.get("id"),
            email: row.get("email"),
            display_name: row.get("display_name"),
            username: row.try_get("username").unwrap_or(None),
            picture_url: row.get("picture_url"),
            phone_number: row.get("phone_number"),
            profile_public: row.try_get("profile_public").unwrap_or(true),
            privacy_settings: row.try_get("privacy_settings").unwrap_or(None),
            role: row.get("role"),
            status: row.get("status"),
            last_login: row.get("last_login"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            deleted_at: row.get("deleted_at"),
            onboarding_completed: row.try_get("onboarding_completed").unwrap_or(false),
            onboarding_goals: row.try_get("onboarding_goals").unwrap_or(None),
            exam_timeframe: row.try_get("exam_timeframe").unwrap_or(None),
            target_exam_date: row.try_get("target_exam_date").unwrap_or(None),
        })
    }
}

// Request to check if a user has a phone number
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CheckPhoneNumberRequest {
    pub user_id: String,
}

// Response for phone number check
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CheckPhoneNumberResponse {
    pub has_phone: bool,
    pub phone_number: Option<String>,
    pub user_id: String,
}

// Request to update a user's phone number
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdatePhoneNumberRequest {
    pub user_id: String,
    pub phone_number: String,
}

// Response for phone number update
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdatePhoneNumberResponse {
    pub success: bool,
    pub message: String,
    pub user_id: String,
    pub phone_number: String,
}

// Admin-specific user model with subscription info
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AdminUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub picture_url: Option<String>,
    pub phone_number: Option<String>,
    pub role: String,
    pub status: String,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub subscription_status: Option<String>,
    pub subscription_end_date: Option<DateTime<Utc>>,
    pub provider: String,
}

impl<'c> FromRow<'c, MySqlRow> for AdminUser {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(AdminUser {
            id: row.get("id"),
            email: row.get("email"),
            display_name: row.get("display_name"),
            picture_url: row.get("picture_url"),
            phone_number: row.get("phone_number"),
            role: row.get("role"),
            status: row.get("status"),
            last_login: row.get("last_login"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            subscription_status: row.get("subscription_status"),
            subscription_end_date: row.get("subscription_end_date"),
            provider: row.get("provider"),
        })
    }
}

impl<'c> FromRow<'c, MySqlRow> for UserStats {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(UserStats {
            total_users: row.get("total_users"),
            active_users: row.get("active_users"),
            admin_users: row.get("admin_users"),
            users_with_premium: row.get("users_with_premium"),
            users_registered_today: row.get("users_registered_today"),
            users_registered_this_month: row.get("users_registered_this_month"),
        })
    }
}

// Request for creating/updating users by admin
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AdminUserRequest {
    pub email: String,
    pub display_name: String,
    pub role: Option<String>,
    pub status: Option<String>,
    pub phone_number: Option<String>,
}

// Response for user statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserStats {
    pub total_users: i64,
    pub active_users: i64,
    pub admin_users: i64,
    pub users_with_premium: i64,
    pub users_registered_today: i64,
    pub users_registered_this_month: i64,
}

// Request for user search/filtering
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserSearchRequest {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub search: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    pub has_premium: Option<bool>,
}

// Paginated user response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedUsersResponse {
    pub users: Vec<AdminUser>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserProfileResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub username: Option<String>,
    pub picture_url: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub account_status: String, // "Free" | "Premium"
    pub premium_expires_at: Option<DateTime<Utc>>,
    pub onboarding_completed: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OnboardingRequest {
    pub goals: Vec<String>,
    pub timeframe: Option<String>,
    pub exam_date: Option<NaiveDate>,
    pub onboarding_completed: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OnboardingResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecommendedPackage {
    pub id: i32,
    pub name: String,
    pub category: String,
    pub question_count: i64,
    pub is_free: bool,
    pub is_premium: bool,
}

impl<'c> FromRow<'c, MySqlRow> for RecommendedPackage {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(RecommendedPackage {
            id: row.get("id"),
            name: row.get("name"),
            category: row.get("category"),
            question_count: row.get("question_count"),
            is_free: row.get("is_free"),
            is_premium: row.get("is_premium"),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecommendationsResponse {
    pub data: Vec<RecommendedPackage>,
    pub based_on_goals: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserLearningStatsResponse {
    pub total_quizzes: i64,
    pub avg_score: f64,
    pub favorite_category: Option<String>,
    pub learning_streak_days: i64,
    pub total_correct: i64,
    pub total_questions: i64,
}