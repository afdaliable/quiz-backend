use supabase_auth::models::{SignUpWithPasswordOptions, AuthClient};

use crate::dao::Database;
use crate::config::Config;
use crate::service::redis_service::RedisPool;
use crate::service::ai_service::AiService;
use std::sync::{Arc, Mutex};
//use sqlx::MySqlPool;

pub mod config;
pub mod levels;
pub mod controller;
pub mod dao;
pub mod model;
pub mod docs;
pub mod middleware;
pub mod utils;
pub mod service;




// AppState
// This the primary dependency for our application's dependency injection.
// Each controller_test function that interacts with the database will require an `AppState` instance in
// order to communicate with the database.
pub struct AppState<'a> {
    pub connections: Mutex<u32>,
    /// Write pool — always points to the master (NAS).
    pub context: Arc<Database<'a>>,
    /// Read pool — points to the local replica when `dao.read_address` is configured,
    /// otherwise shares the same `Arc` as `context`.
    pub read_context: Arc<Database<'a>>,
    pub config: Arc<Config>,
    pub auth_client: AuthClient,
    pub sign_up_with_password_options: SignUpWithPasswordOptions,
    pub redis_pool: Option<Arc<RedisPool>>,
    pub ai_service: Option<Arc<AiService>>,
}
