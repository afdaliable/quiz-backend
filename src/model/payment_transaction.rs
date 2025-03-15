use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PaymentTransaction {
    pub id: i32,
    pub user_id: String,
    pub plan_id: i32,
    pub amount: f64,
    pub transaction_id: String,
    pub payment_link: String,
    pub status: PaymentStatus,
    pub payment_method: Option<String>,
    pub payment_details: Option<String>, // JSON with payment details
    pub webhook_data: Option<String>,    // JSON with webhook data
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "ENUM", rename_all = "lowercase")]
pub enum PaymentStatus {
    Pending,
    Completed,
    Failed,
    Expired,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentTransactionResponse {
    pub id: i32,
    pub user_id: String,
    pub plan_id: i32,
    pub plan_name: String,
    pub amount: f64,
    pub transaction_id: String,
    pub payment_link: String,
    pub status: String,
    pub payment_method: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePaymentRequest {
    pub user_id: String,
    pub plan_id: i32,
    pub redirect_url: String,
}

// Mayar API request and response models
#[derive(Debug, Serialize, Deserialize)]
pub struct MayarPaymentRequest {
    pub name: String,
    pub email: String,
    pub amount: f64,
    pub mobile: String,
    pub redirectUrl: String,
    pub description: String,
    pub expiredAt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MayarPaymentResponse {
    pub statusCode: i32,
    pub messages: String,
    pub data: MayarPaymentData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MayarPaymentData {
    pub id: String,
    pub transaction_id: String,
    pub transactionId: String,
    pub link: String,
}

// Webhook models
#[derive(Debug, Serialize, Deserialize)]
pub struct MayarWebhookPayload {
    pub event: String,
    pub data: MayarWebhookData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MayarWebhookData {
    pub id: String,
    #[serde(default = "default_transaction_id")]
    pub transactionId: String,
    pub status: String,
    #[serde(default)]
    pub transactionStatus: String,
    #[serde(default)]
    #[serde(with = "chrono_string_or_i64")]
    pub createdAt: i64,
    #[serde(default)]
    #[serde(with = "chrono_string_or_i64")]
    pub updatedAt: i64,
    pub merchantId: String,
    pub merchantName: String,
    pub merchantEmail: String,
    pub customerName: String,
    pub customerEmail: String,
    pub customerMobile: String,
    pub amount: f64,
    #[serde(default)]
    pub isAdminFeeBorneByCustomer: bool,
    #[serde(default)]
    pub isChannelFeeBorneByCustomer: bool,
    pub productId: String,
    pub productName: String,
    pub productType: String,
    #[serde(default)]
    pub pixelFbp: Option<String>,
    #[serde(default)]
    pub pixelFbc: Option<String>,
    #[serde(default)]
    pub paymentUrl: Option<String>,
    // Add support for custom fields
    #[serde(default)]
    pub custom_field: Option<Vec<serde_json::Value>>,
    // New fields from real webhook
    #[serde(default)]
    pub couponUsed: Option<String>,
    #[serde(default)]
    pub paymentMethod: Option<String>,
    #[serde(default)]
    pub nettAmount: Option<f64>,
}

// Default function for transactionId
fn default_transaction_id() -> String {
    "test-transaction-id".to_string()
}

// Module to handle both string and i64 formats for dates
mod chrono_string_or_i64 {
    use serde::{self, Deserialize, Serializer, Deserializer};
    use serde::de::{self, Visitor};
    use std::fmt;

    pub fn serialize<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(*value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrI64Visitor;

        impl<'de> Visitor<'de> for StringOrI64Visitor {
            type Value = i64;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string or i64")
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(value)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // Try to parse as RFC3339 date string
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
                    return Ok(dt.timestamp_millis());
                }
                
                // If not a date string, try to parse as i64
                value.parse::<i64>().map_err(de::Error::custom)
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(StringOrI64Visitor)
    }
} 