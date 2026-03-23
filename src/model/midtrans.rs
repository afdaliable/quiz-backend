use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ─── DB model ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MidtransTransaction {
    pub id: String,
    pub user_id: String,
    pub subscription_id: Option<i32>,
    pub plan_id: i32,
    pub order_id: String,
    pub amount: i64,
    pub status: String,
    pub payment_method: Option<String>,
    pub payment_url: Option<String>,
    pub snap_token: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub midtrans_transaction_id: Option<String>,
    pub midtrans_status_code: Option<String>,
    pub invoice_number: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ─── Midtrans Snap API response ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapResponse {
    pub token: String,
    pub redirect_url: String,
}

// ─── Request / Response DTOs ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CheckoutRequest {
    /// Plan slug: "silver" | "gold" | "platinum" | "ultimate"
    pub plan: String,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub order_id: String,
    pub payment_url: String,
    pub snap_token: String,
    pub amount: i64,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionStatusResponse {
    pub plan: Option<String>,
    pub plan_name: Option<String>,
    pub status: String, // "active" | "expired" | "none"
    pub started_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub days_remaining: Option<i64>,
    pub is_lifetime: bool,
    pub price_paid: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TransactionHistoryItem {
    pub id: String,
    pub plan: String,
    pub plan_name: String,
    pub amount: i64,
    pub status: String,
    pub payment_method: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub invoice_number: Option<String>,
    pub invoice_url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TransactionHistoryResponse {
    pub data: Vec<TransactionHistoryItem>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

// ─── Midtrans webhook notification ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MidtransNotification {
    pub order_id: String,
    pub status_code: String,
    pub gross_amount: String,
    pub signature_key: String,
    pub transaction_status: String,
    pub payment_type: Option<String>,
    pub transaction_id: Option<String>,
    pub fraud_status: Option<String>,
}
