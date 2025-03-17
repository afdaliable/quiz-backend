use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Clone)]
struct AppConfig {
    url: String,
    port: u16,
}
#[derive(Deserialize, Clone)]
struct DaoConfig {
    user: String,
    password: String,
    address: String,
    database: String,
}

#[derive(Deserialize, Clone)]
struct GoogleOAuthConfig {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

#[derive(Deserialize, Clone)]
struct PaymentConfig {
    mayar_api_key: String,
    mayar_api_url: String,
    mayar_webhook_url: String,
    mayar_webhook_secret: String,
    mayar_saas_api_url: String,
}

#[derive(Deserialize, Clone)]
pub struct Config {
    app: AppConfig,
    dao: DaoConfig,
    api_key: String,
    jwt_secret: String,
    anon_key: String,
    auth_url: String,
    google_oauth: GoogleOAuthConfig,
    payment: PaymentConfig,
}

impl Config {
    pub fn from_file(path: &'static str) -> Self {
        let config = fs::read_to_string(path).unwrap();
        serde_json::from_str(&config).unwrap()
    }

    pub fn get_app_url(&self) -> String {
        format!("{0}:{1}", self.app.url, self.app.port)
    }

    pub fn get_database_url(&self) -> String {
        format!(
            "mysql://{0}:{1}@{2}/{3}",
            self.dao.user, self.dao.password, self.dao.address, self.dao.database
        )
    }

    pub fn get_api_key(&self) -> &str {
        &self.api_key
    }

    pub fn get_jwt_secret(&self) -> &str {
        &self.jwt_secret
    }

    pub fn get_anon_key(&self) -> &str {
        &self.anon_key
    }

    pub fn get_auth_url(&self) -> &str {
        &self.auth_url
    }
    
    pub fn get_google_client_id(&self) -> &str {
        &self.google_oauth.client_id
    }

    pub fn get_google_client_secret(&self) -> &str {
        &self.google_oauth.client_secret
    }

    pub fn get_google_redirect_uri(&self) -> &str {
        &self.google_oauth.redirect_uri
    }

    pub fn get_mayar_api_key(&self) -> &str {
        &self.payment.mayar_api_key
    }

    pub fn get_mayar_api_url(&self) -> &str {
        &self.payment.mayar_api_url
    }

    pub fn get_mayar_webhook_url(&self) -> &str {
        &self.payment.mayar_webhook_url
    }

    pub fn get_mayar_webhook_secret(&self) -> &str {
        &self.payment.mayar_webhook_secret
    }

    pub fn get_mayar_saas_api_url(&self) -> &str {
        &self.payment.mayar_saas_api_url
    }
}
