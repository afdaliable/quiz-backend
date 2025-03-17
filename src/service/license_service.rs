use crate::model::license_code::{MayarLicenseVerifyRequest, MayarLicenseVerifyResponse};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::error::Error;
use log::{info, error, debug};

pub struct LicenseService {
    api_key: String,
    api_url: String,
}

impl LicenseService {
    pub fn new(api_key: String, api_url: String) -> Self {
        LicenseService { api_key, api_url }
    }

    pub async fn verify_license(&self, license_code: &str, product_id: &str) -> Result<MayarLicenseVerifyResponse, Box<dyn Error>> {
        // Construct the full URL
        let full_url = format!("{}/license/verify", self.api_url);
        
        // Log the URL for debugging
        info!("Verifying license at: {}", full_url);
        debug!("License code: {}, Product ID: {}", license_code, product_id);
        
        // Create request headers
        let mut headers = HeaderMap::new();
        match HeaderValue::from_str(&format!("Bearer {}", self.api_key)) {
            Ok(value) => {
                headers.insert(AUTHORIZATION, value);
            },
            Err(e) => {
                error!("Failed to create Authorization header: {}", e);
                return Err(format!("Failed to create Authorization header: {}", e).into());
            }
        }
        
        match HeaderValue::from_str("application/json") {
            Ok(value) => {
                headers.insert(CONTENT_TYPE, value);
            },
            Err(e) => {
                error!("Failed to create Content-Type header: {}", e);
                return Err(format!("Failed to create Content-Type header: {}", e).into());
            }
        }

        // Create request body
        let request_body = MayarLicenseVerifyRequest {
            licenseCode: license_code.to_string(),
            productId: product_id.to_string(),
        };

        // Log the request body for debugging
        match serde_json::to_string(&request_body) {
            Ok(body_str) => {
                info!("Request body: {}", body_str);
            },
            Err(e) => {
                error!("Failed to serialize request body: {}", e);
                return Err(format!("Failed to serialize request body: {}", e).into());
            }
        }

        // Send the request
        info!("Sending request to Mayar API");
        let client = reqwest::Client::new();
        let response = match client
            .post(&full_url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Failed to send request to Mayar API: {}", e);
                    return Err(format!("Failed to send request to Mayar API: {}", e).into());
                }
            };

        // Get the status code
        let status = response.status();
        info!("Mayar API response status: {}", status);
        
        // Get the response body
        let body = match response.text().await {
            Ok(body_text) => body_text,
            Err(e) => {
                error!("Failed to get response body: {}", e);
                return Err(format!("Failed to get response body: {}", e).into());
            }
        };
        info!("Mayar API response body: {}", body);

        if status.is_success() {
            match serde_json::from_str::<MayarLicenseVerifyResponse>(&body) {
                Ok(license_response) => {
                    info!("Successfully parsed Mayar API response");
                    Ok(license_response)
                },
                Err(e) => {
                    error!("Failed to parse Mayar API response: {}, body: {}", e, body);
                    Err(format!("Failed to parse response: {}, body: {}", e, body).into())
                }
            }
        } else {
            error!("License verification failed with status {}: {}", status, body);
            Err(format!("License verification failed: {}", body).into())
        }
    }
} 