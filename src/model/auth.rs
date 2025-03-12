use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request payload for user signup
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SignUpRequest {
    /// User's email address
    pub email: String,
    /// User's password
    pub password: String,
    /// User's display name
    pub display_name: String,
}

/// Request payload for user login
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// User's email address
    pub email: String,
    /// User's password
    pub password: String,
}

/// Response containing authentication details
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthResponse {
    /// JWT access token
    pub access_token: String,
    /// Token type (usually "bearer")
    pub token_type: String,
    /// Token expiration time in seconds
    pub expires_in: i32,
    /// Refresh token for obtaining new access tokens
    pub refresh_token: String,
    /// User information
    pub user: SupabaseUser,
}

/// Supabase user information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SupabaseUser {
    /// User's unique identifier
    pub id: String,
    /// User's email address
    pub email: String,
    /// User's display name
    pub display_name: String,
    /// User's profile picture URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
    // Add other fields as needed
}
