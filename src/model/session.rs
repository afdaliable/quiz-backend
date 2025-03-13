use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub user_id: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl<'c> FromRow<'c, MySqlRow> for Session {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Session {
            id: row.get("id"),
            user_id: row.get("user_id"),
            token: row.get("token"),
            expires_at: row.get("expires_at"),
            ip_address: row.get("ip_address"),
            user_agent: row.get("user_agent"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionResponse {
    pub token: String,
    pub expires_at: DateTime<Utc>,
} 