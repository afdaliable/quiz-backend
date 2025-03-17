use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct LicenseCode {
    pub id: i32,
    pub license_code: String,
    pub user_id: Option<String>,
    pub plan_id: i32,
    pub status: LicenseStatus,
    pub transaction_id: Option<String>,
    pub product_id: String,
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub expired_at: Option<DateTime<Utc>>,
    pub activation_limit: Option<String>,
    pub use_count: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub license_data: Option<String>, // Full JSON response from Mayar
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "ENUM", rename_all = "lowercase")]
pub enum LicenseStatus {
    #[sqlx(rename = "active")]
    Active,
    #[sqlx(rename = "expired")]
    Expired,
    #[sqlx(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LicenseCodeResponse {
    pub id: i32,
    pub license_code: String,
    pub user_id: Option<String>,
    pub plan_id: i32,
    pub plan_name: String,
    pub status: String,
    pub expired_at: Option<DateTime<Utc>>,
    pub days_remaining: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLicenseCodeRequest {
    pub license_code: String,
    pub user_id: String,
    pub product_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyLicenseRequest {
    pub license_code: String,
    pub product_id: String,
    pub email: String,
    pub name: String,
    pub phone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MayarLicenseVerifyRequest {
    pub licenseCode: String,
    pub productId: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MayarLicenseVerifyResponse {
    pub statusCode: i32,
    pub isLicenseActive: bool,
    pub licenseCode: MayarLicenseData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MayarLicenseData {
    pub licenseCode: String,
    pub status: String,
    pub expiredAt: Option<String>,
    pub transactionId: String,
    pub productId: String,
    pub customerId: Option<String>,
    pub customerName: Option<String>,
    pub customerEmail: Option<String>,
    pub activationLimit: Option<String>,
    pub useCount: Option<i32>,
    pub createdAt: Option<String>,
    pub updatedAt: Option<String>,
    pub membershipTierId: Option<String>,
    pub membershipTierName: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratePaymentLinkRequest {
    pub plan_id: i32,
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratePaymentLinkResponse {
    pub payment_link: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivateLicenseResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
    pub user: Option<UserInfo>,
    pub subscription: Option<SubscriptionInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    pub id: i32,
    pub plan_id: i32,
    pub plan_name: String,
    pub expired_at: Option<DateTime<Utc>>,
    pub is_lifetime: bool,
} 