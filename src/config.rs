use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Clone)]
pub struct AiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_secs: u64,
}

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
    /// Optional read-replica address (host:port). Falls back to `address` if absent.
    read_address: Option<String>,
}

#[derive(Deserialize, Clone)]
struct GoogleOAuthConfig {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

#[derive(Deserialize, Clone, Default)]
struct MidtransConfig {
    #[serde(default)]
    server_key: String,
    #[serde(default)]
    client_key: String,
    #[serde(default)]
    is_production: bool,
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
struct RedisConfig {
    host: String,
    port: u16,
    password: String,
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
    redis: RedisConfig,
    internal_api_key: Option<String>,
    #[serde(default)]
    midtrans: MidtransConfig,
    #[serde(default)]
    ai: Option<AiConfig>,
    #[serde(default)]
    upload_dir: Option<String>,
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

    /// Returns the URL for the read replica. If no `read_address` is configured,
    /// returns the same URL as `get_database_url()` so the caller can share the pool.
    pub fn get_read_database_url(&self) -> String {
        let read_address = self.dao.read_address.as_deref()
            .unwrap_or(&self.dao.address);
        format!(
            "mysql://{0}:{1}@{2}/{3}",
            self.dao.user, self.dao.password, read_address, self.dao.database
        )
    }

    pub fn has_read_replica(&self) -> bool {
        self.dao.read_address.is_some()
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

    pub fn get_redis_url(&self) -> String {
        format!("redis://:{}@{}:{}/", self.redis.password, self.redis.host, self.redis.port)
    }

    pub fn get_redis_host(&self) -> &str {
        &self.redis.host
    }

    pub fn get_redis_port(&self) -> u16 {
        self.redis.port
    }

    pub fn get_redis_password(&self) -> &str {
        &self.redis.password
    }

    pub fn get_internal_api_key(&self) -> Option<&str> {
        self.internal_api_key.as_deref()
    }

    pub fn get_midtrans_server_key(&self) -> &str {
        &self.midtrans.server_key
    }

    pub fn get_midtrans_client_key(&self) -> &str {
        &self.midtrans.client_key
    }

    pub fn get_midtrans_is_production(&self) -> bool {
        self.midtrans.is_production
    }

    pub fn get_ai_config(&self) -> Option<&AiConfig> {
        self.ai.as_ref()
    }

    pub fn get_upload_dir(&self) -> String {
        self.upload_dir.clone().unwrap_or_else(|| "./uploads".to_string())
    }
}
