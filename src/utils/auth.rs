use actix_web::HttpRequest;
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,     // Subject (user ID)
    pub email: String,   // User email
    pub name: String,    // User name
    pub exp: usize,      // Expiration time
    pub iat: usize,      // Issued at
    pub aud: String,     // Audience
    pub iss: String,     // Issuer
}

/// Extract user ID from the Authorization header
pub fn get_user_id_from_token(req: &HttpRequest) -> Option<String> {
    // First try to get from user_id header (for backward compatibility)
    if let Some(user_id) = req.headers().get("user_id") {
        if let Ok(id) = user_id.to_str() {
            return Some(id.to_string());
        }
    }

    // If not found, try to extract from Authorization header
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = auth_str.trim_start_matches("Bearer ").trim();
                
                // Get JWT secret from app data
                if let Some(app_state) = req.app_data::<actix_web::web::Data<crate::AppState>>() {
                    let jwt_secret = app_state.config.get_jwt_secret();
                    
                    // Decode the token
                    let token_data = decode::<TokenClaims>(
                        token,
                        &DecodingKey::from_secret(jwt_secret.as_bytes()),
                        &Validation::new(Algorithm::HS256),
                    );
                    
                    if let Ok(data) = token_data {
                        return Some(data.claims.sub);
                    }
                }
            }
        }
    }
    
    None
}

/// Extract user ID from the request
/// This is a wrapper around get_user_id_from_token for better naming
pub fn extract_user_id(req: &HttpRequest) -> Option<String> {
    get_user_id_from_token(req)
}

/// Generate a JWT token for a user
pub fn generate_token(user_id: &str, email: &str, name: &str, jwt_secret: &str, app_url: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::days(30)).timestamp() as usize; // Token valid for 30 days
    
    let claims = TokenClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        name: name.to_string(),
        exp,
        iat,
        aud: app_url.to_string(), // Set audience from app_url
        iss: app_url.to_string(), // Set issuer from app_url
    };
    
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
} 