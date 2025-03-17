use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub picture_url: Option<String>,
    pub phone_number: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl<'c> FromRow<'c, MySqlRow> for User {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(User {
            id: row.get("id"),
            email: row.get("email"),
            display_name: row.get("display_name"),
            picture_url: row.get("picture_url"),
            phone_number: row.get("phone_number"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            deleted_at: row.get("deleted_at"),
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