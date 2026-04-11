use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AdminAlert {
    pub id: String,
    #[serde(rename = "type")]
    pub alert_type: String,
    pub entity_type: String,
    pub entity_id: String,
    /// JSON object with alert details
    pub detail: serde_json::Value,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

impl<'c> FromRow<'c, MySqlRow> for AdminAlert {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        let detail_str: String = row.try_get("detail").unwrap_or_else(|_| "{}".to_string());
        let detail: serde_json::Value = serde_json::from_str(&detail_str)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        Ok(AdminAlert {
            id: row.get("id"),
            alert_type: row.get("type"),
            entity_type: row.get("entity_type"),
            entity_id: row.get("entity_id"),
            detail,
            is_read: row.get("is_read"),
            created_at: row.get("created_at"),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AlertsQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    /// Filter by read status: "read" | "unread" | "all" (default: "all")
    pub status: Option<String>,
    /// Filter by alert type
    #[serde(rename = "type")]
    pub alert_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedAlertsResponse {
    pub alerts: Vec<AdminAlert>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}
