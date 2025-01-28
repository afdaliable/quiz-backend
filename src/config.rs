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
pub struct Config {
    app: AppConfig,
    dao: DaoConfig,
    api_key: String,
    jwt_secret: String,
    anon_key: String,
    auth_url: String,
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
}
