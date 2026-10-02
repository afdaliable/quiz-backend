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
pub struct GoogleOAuthConfig {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl GoogleOAuthConfig {
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn client_secret(&self) -> &str {
        &self.client_secret
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }
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

/// Pembayaran QRIS statis milik merchant sendiri (GoPay Merchant dll).
/// `static_payload` adalah isi QR statis apa adanya, diawali "00020101021126...".
#[derive(Deserialize, Clone)]
pub struct QrisConfig {
    pub static_payload: String,
    /// Jam sampai klaim yang belum disetujui dicabut otomatis. 0 = tanpa batas.
    #[serde(default = "default_auto_revoke")]
    pub auto_revoke_hours: u32,
}

fn default_auto_revoke() -> u32 {
    48
}

/// Payment gateway KlikQRIS (QRIS dinamis + webhook).
#[derive(Deserialize, Clone)]
pub struct KlikqrisConfig {
    #[serde(default = "default_klikqris_base")]
    pub base_url: String,
    pub api_key: String,
    pub id_merchant: String,
    /// URL webhook per transaksi. Kosong = pakai webhook global di dashboard KlikQRIS.
    #[serde(default)]
    pub callback_url: Option<String>,
}

fn default_klikqris_base() -> String {
    "https://klikqris.com/api".to_string()
}

/// Notifikasi ke pemilik saat ada klaim pembayaran, lewat bot Telegram.
#[derive(Deserialize, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    /// Chat tujuan notifikasi (chat pribadi pemilik atau grup).
    pub chat_id: String,
    /// Token rahasia yang dicocokkan dengan header
    /// `X-Telegram-Bot-Api-Secret-Token` pada webhook, supaya orang lain tidak
    /// bisa memalsukan persetujuan.
    pub webhook_secret: String,
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
    /// Raw JSON for the "ai" section -- kept untyped here so a shape mismatch
    /// (e.g. an instance still running an older config.json after an AiConfig
    /// field rename) can't crash the whole config load. `from_file` converts
    /// this into the typed `ai` field, degrading to None with a warning
    /// instead of panicking on a bad/stale "ai" section.
    #[serde(default, rename = "ai")]
    ai_raw: Option<serde_json::Value>,
    #[serde(skip)]
    ai: Option<AiConfig>,
    #[serde(default)]
    upload_dir: Option<String>,
    #[serde(default)]
    google_oauth_upkp: Option<GoogleOAuthConfig>,
    #[serde(default)]
    qris: Option<QrisConfig>,
    #[serde(default)]
    telegram: Option<TelegramConfig>,
    #[serde(default)]
    klikqris: Option<KlikqrisConfig>,
}

impl Config {
    pub fn from_file(path: &'static str) -> Self {
        let raw = fs::read_to_string(path).unwrap();
        let mut config: Config = serde_json::from_str(&raw).unwrap();
        config.ai = config.ai_raw.take().and_then(|v| {
            match serde_json::from_value::<AiConfig>(v) {
                Ok(ai) => Some(ai),
                Err(e) => {
                    eprintln!(
                        "Warning: 'ai' config section present but doesn't match AiConfig ({}) -- AI features disabled on this instance",
                        e
                    );
                    None
                }
            }
        });
        config
    }

    pub fn get_qris(&self) -> Option<&QrisConfig> {
        self.qris.as_ref()
    }

    pub fn get_klikqris(&self) -> Option<&KlikqrisConfig> {
        self.klikqris.as_ref()
    }

    pub fn get_telegram(&self) -> Option<&TelegramConfig> {
        self.telegram.as_ref()
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

    pub fn get_google_oauth_upkp(&self) -> Option<&GoogleOAuthConfig> {
        self.google_oauth_upkp.as_ref()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_config_json(ai_section: &str) -> String {
        format!(
            r#"{{
                "app": {{"url": "http://localhost", "port": 8080}},
                "dao": {{"user": "u", "password": "p", "address": "a", "database": "d"}},
                "api_key": "k",
                "jwt_secret": "s",
                "anon_key": "a",
                "auth_url": "u",
                "google_oauth": {{"client_id": "c", "client_secret": "s", "redirect_uri": "r"}},
                "payment": {{"mayar_api_key": "k", "mayar_api_url": "u", "mayar_webhook_url": "u", "mayar_webhook_secret": "s", "mayar_saas_api_url": "u"}},
                "redis": {{"host": "h", "port": 6379, "password": "p"}}
                {ai_section}
            }}"#
        )
    }

    fn write_temp(content: &str) -> &'static str {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = format!("/tmp/config_test_{}_{}.json", std::process::id(), n);
        std::fs::write(&path, content).unwrap();
        Box::leak(path.into_boxed_str())
    }

    #[test]
    fn missing_ai_section_loads_fine() {
        let path = write_temp(&minimal_config_json(""));
        let cfg = Config::from_file(path);
        assert!(cfg.get_ai_config().is_none());
    }

    #[test]
    fn valid_ai_section_loads() {
        let ai = r#", "ai": {"base_url": "http://x", "api_key": "k", "model": "m", "max_tokens": 100, "temperature": 0.5, "timeout_secs": 10}"#;
        let path = write_temp(&minimal_config_json(ai));
        let cfg = Config::from_file(path);
        let ai_cfg = cfg.get_ai_config().expect("ai config should parse");
        assert_eq!(ai_cfg.model, "m");
    }

    /// The regression this guards: an "ai" section in the old shape (as a
    /// stale replica's config.json might still have after AiConfig's fields
    /// were renamed) must degrade to None, not panic the whole config load.
    #[test]
    fn stale_ai_section_shape_degrades_instead_of_panicking() {
        let ai = r#", "ai": {"primary_provider": "deepseek", "deepseek_api_key": "x", "deepseek_model": "y", "gemini_api_key": "z", "gemini_model": "w", "fallback_enabled": true, "max_tokens": 100, "temperature": 0.5, "timeout_secs": 10}"#;
        let path = write_temp(&minimal_config_json(ai));
        let cfg = Config::from_file(path);
        assert!(cfg.get_ai_config().is_none());
    }
}
