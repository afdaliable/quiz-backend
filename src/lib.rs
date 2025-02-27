use supabase_auth::models::{SignUpWithPasswordOptions, AuthClient};

use crate::dao::Database;
use crate::config::Config;
use std::sync::{Arc, Mutex};
//use sqlx::MySqlPool;

pub mod config;
pub mod controller;
pub mod dao;
pub mod model;
pub mod docs;
pub mod middleware;




// AppState
// This the primary dependency for our application's dependency injection.
// Each controller_test function that interacts with the database will require an `AppState` instance in
// order to communicate with the database.
pub struct AppState<'a> {
    pub connections: Mutex<u32>,
    pub context: Arc<Database<'a>>,
    pub config: Arc<Config>,
    pub auth_client: AuthClient,
    pub sign_up_with_password_options: SignUpWithPasswordOptions,
}
