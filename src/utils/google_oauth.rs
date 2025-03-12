use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenUrl, AuthorizationCode, reqwest::async_http_client,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use reqwest::Client;
use std::error::Error as StdError;

#[derive(Error, Debug)]
pub enum GoogleOAuthError {
    #[error("OAuth2 error: {0}")]
    OAuth2Error(#[from] Box<dyn StdError + Send + Sync>),
    #[error("HTTP request error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("JSON serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Invalid token")]
    InvalidToken,
    #[error("User info not found")]
    UserInfoNotFound,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub locale: Option<String>,
}

pub struct GoogleOAuthClient {
    client: BasicClient,
    http_client: Client,
}

impl GoogleOAuthClient {
    pub fn new(client_id: &str, client_secret: &str, redirect_uri: &str) -> Self {
        let google_client_id = ClientId::new(client_id.to_string());
        let google_client_secret = ClientSecret::new(client_secret.to_string());
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .expect("Invalid authorization endpoint URL");
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .expect("Invalid token endpoint URL");

        let client = BasicClient::new(
            google_client_id,
            Some(google_client_secret),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string()).expect("Invalid redirect URL"));

        Self {
            client,
            http_client: Client::new(),
        }
    }

    pub fn generate_auth_url(&self) -> (String, CsrfToken) {
        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .url();

        (auth_url.to_string(), csrf_token)
    }

    pub async fn exchange_code_for_token(
        &self,
        code: &str,
    ) -> Result<oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>, GoogleOAuthError> {
        let token = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| GoogleOAuthError::OAuth2Error(Box::new(e)))?;

        Ok(token)
    }

    pub async fn get_user_info(&self, access_token: &str) -> Result<GoogleUserInfo, GoogleOAuthError> {
        let user_info = self
            .http_client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token)
            .send()
            .await?
            .json::<GoogleUserInfo>()
            .await?;

        Ok(user_info)
    }
} 