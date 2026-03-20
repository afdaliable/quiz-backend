use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::Client;
use serde_json::json;
use sha2::{Digest, Sha512};

use crate::model::midtrans::SnapResponse;

pub struct MidtransService {
    server_key: String,
    client_key: String,
    is_production: bool,
    http: Client,
}

impl MidtransService {
    pub fn new(server_key: String, client_key: String, is_production: bool) -> Self {
        MidtransService {
            server_key,
            client_key,
            is_production,
            http: Client::new(),
        }
    }

    fn snap_url(&self) -> &'static str {
        if self.is_production {
            "https://app.midtrans.com/snap/v1/transactions"
        } else {
            "https://app.sandbox.midtrans.com/snap/v1/transactions"
        }
    }

    fn auth_header(&self) -> String {
        let encoded = STANDARD.encode(format!("{}:", self.server_key));
        format!("Basic {}", encoded)
    }

    pub fn get_client_key(&self) -> &str {
        &self.client_key
    }

    /// POST ke Midtrans Snap API dan kembalikan token + redirect_url
    pub async fn create_snap_transaction(
        &self,
        order_id: &str,
        amount: i64,
        user_email: &str,
        user_name: &str,
        plan_slug: &str,
        plan_name: &str,
    ) -> Result<SnapResponse, reqwest::Error> {
        let payload = json!({
            "transaction_details": {
                "order_id": order_id,
                "gross_amount": amount,
            },
            "customer_details": {
                "first_name": user_name,
                "email": user_email,
            },
            "item_details": [{
                "id": plan_slug,
                "price": amount,
                "quantity": 1,
                "name": plan_name,
            }],
            "expiry": {
                "duration": 24,
                "unit": "hours",
            },
        });

        self.http
            .post(self.snap_url())
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?
            .json::<SnapResponse>()
            .await
    }

    /// Verifikasi signature Midtrans:
    /// SHA512(order_id + status_code + gross_amount + server_key)
    pub fn verify_signature(
        &self,
        order_id: &str,
        status_code: &str,
        gross_amount: &str,
        received: &str,
    ) -> bool {
        let raw = format!("{}{}{}{}", order_id, status_code, gross_amount, self.server_key);
        let mut hasher = Sha512::new();
        hasher.update(raw.as_bytes());
        let computed = hex::encode(hasher.finalize());
        computed == received
    }
}
