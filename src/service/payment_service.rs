use crate::model::payment_transaction::{MayarPaymentRequest, MayarPaymentResponse, PaymentStatus};
use chrono::{DateTime, Duration, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;
use std::error::Error;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct MayarPaymentService {
    api_key: String,
    api_url: String,
}

impl MayarPaymentService {
    pub fn new(api_key: String, api_url: String) -> Self {
        MayarPaymentService { api_key, api_url }
    }

    pub async fn create_payment(
        &self,
        name: &str,
        email: &str,
        amount: f64,
        mobile: &str,
        redirect_url: &str,
        description: &str,
    ) -> Result<MayarPaymentResponse, Box<dyn Error>> {
        let client = reqwest::Client::new();
        
        let full_url = format!("{}/payment/create", self.api_url);
        println!("Making request to Mayar API: {}", full_url);
        
        let request_body = MayarPaymentRequest {
            name: name.to_string(),
            email: email.to_string(),
            amount,
            mobile: mobile.to_string(),
            redirectUrl: redirect_url.to_string(),
            description: description.to_string(),
            expiredAt: (Utc::now() + Duration::hours(24)).to_rfc3339(),
        };
        
        println!("Request payload: {:?}", request_body);
        println!("Using API key: {}", self.api_key.chars().take(5).collect::<String>() + "...");
        
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = client
            .post(&full_url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        println!("Mayar API response status: {}", status);
        println!("Mayar API response body: {}", body);

        if status.is_success() {
            match serde_json::from_str::<MayarPaymentResponse>(&body) {
                Ok(payment_response) => Ok(payment_response),
                Err(e) => Err(format!("Failed to parse response: {}, body: {}", e, body).into())
            }
        } else {
            Err(format!("Payment creation failed: {}", body).into())
        }
    }

    pub async fn check_payment_status(&self, transaction_id: &str) -> Result<PaymentStatus, Box<dyn Error>> {
        // Construct the full URL correctly
        let full_url = format!("{}/payment/{}", self.api_url, transaction_id);
        
        // Log the URL for debugging
        println!("Checking payment status at: {}", full_url);
        
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))?,
        );

        let response = client
            .get(&full_url)
            .headers(headers)
            .send()
            .await?;

        if response.status().is_success() {
            let response_json: serde_json::Value = response.json().await?;
            
            if let Some(data) = response_json.get("data") {
                if let Some(status) = data.get("status") {
                    let status_str = status.as_str().unwrap_or("").to_uppercase();
                    
                    return match status_str.as_str() {
                        "SUCCESS" => Ok(PaymentStatus::Completed),
                        "EXPIRED" => Ok(PaymentStatus::Expired),
                        "FAILED" => Ok(PaymentStatus::Failed),
                        _ => Ok(PaymentStatus::Pending),
                    };
                }
            }
            
            Err("Invalid response format".into())
        } else {
            let error_text = response.text().await?;
            Err(format!("Payment status check failed: {}", error_text).into())
        }
    }
} 