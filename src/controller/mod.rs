use super::AppState;
use std::sync::Mutex;
use actix_web::web;

// pub mod group_controller;
// pub mod index_controller;
pub mod soal_controller;
pub mod auth_controller;
pub mod kategori_controller;
pub mod premium_controller;
pub mod payment_controller;
pub mod user_controller;
pub mod license_controller;
pub mod quiz_session_controller;

// pub use group_controller::init as init_group_controller;
// pub use index_controller::init as init_index_controller;
pub use soal_controller::init as init_soal_controller;
pub use auth_controller::init as init_auth_controller;
pub use kategori_controller::init as init_kategori_controller;
pub use premium_controller::init as init_premium_controller;
pub use payment_controller::init as init_payment_controller;
pub use user_controller::init as init_user_controller;
pub use license_controller::init as init_license_controller;

fn log_request(route: &'static str, connections: &Mutex<u32>) {
    let mut con = connections.lock().unwrap();
    *con += 1;
    println!("{}\n\tconnections: {}", route, con);
}


