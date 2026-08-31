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
pub mod bookmark_controller;

// Admin controllers
pub mod admin_user_controller;
pub mod admin_kategori_controller;
pub mod admin_soal_controller;
pub mod admin_packages_controller;
pub mod admin_paket_soal_items_controller;
pub mod admin_analytics_controller;
pub mod admin_passage_controller;
pub mod materi_generate_controller;
pub mod materi_parse_controller;
pub mod taxonomy_classify_controller;
pub mod soal_quality_controller;
pub mod admin_soal_analytics_controller;

// Internal endpoints (IP + API key restricted, no OAuth)
pub mod internal_soal_controller;

pub mod question_comment_controller;
pub mod subscription_controller;
pub mod analytics_controller;
pub mod public_profile_controller;
pub mod search_controller;
pub mod question_feedback_controller;
pub mod leaderboard_controller;
pub mod taxonomy_controller;
pub mod admin_alerts_controller;

// AI enrichment controller (admin-only)
pub mod ai_controller;
pub mod admin_hierarchy_controller;

// AFD-157 Simulasi Ujian
pub mod admin_simulasi_controller;
pub mod simulasi_ujian_controller;

// pub use group_controller::init as init_group_controller;
// pub use index_controller::init as init_index_controller;
pub use soal_controller::init as init_soal_controller;
pub use auth_controller::init as init_auth_controller;
pub use kategori_controller::init as init_kategori_controller;
pub use premium_controller::init as init_premium_controller;
pub use payment_controller::init as init_payment_controller;
pub use user_controller::init as init_user_controller;
pub use license_controller::init as init_license_controller;
pub use bookmark_controller::init as init_bookmark_controller;

// Admin controller exports
pub use admin_user_controller::init as init_admin_user_controller;
pub use admin_kategori_controller::init as init_admin_kategori_controller;
pub use admin_soal_controller::init as init_admin_soal_controller;
pub use admin_packages_controller::init as init_admin_packages_controller;
pub use admin_paket_soal_items_controller::init as init_admin_paket_soal_items_controller;
pub use admin_analytics_controller::init as init_admin_analytics_controller;
pub use admin_alerts_controller::init as init_admin_alerts_controller;
pub use admin_passage_controller::init as init_admin_passage_controller;
pub use materi_generate_controller::init as init_materi_generate_controller;
pub use materi_parse_controller::init as init_materi_parse_controller;
pub use taxonomy_classify_controller::init as init_taxonomy_classify_controller;
pub use soal_quality_controller::init as init_soal_quality_controller;
pub use admin_soal_analytics_controller::init as init_soal_analytics_controller;

// Internal endpoint exports
pub use internal_soal_controller::init as init_internal_soal_controller;

pub use question_comment_controller::init as init_question_comment_controller;
pub use subscription_controller::init as init_subscription_controller;
pub use analytics_controller::init as init_analytics_controller;
pub use ai_controller::init as init_ai_controller;
pub use admin_hierarchy_controller::init as init_admin_hierarchy_controller;
pub use admin_simulasi_controller::init as init_admin_simulasi_controller;

fn log_request(route: &'static str, connections: &Mutex<u32>) {
    let mut con = connections.lock().unwrap();
    *con += 1;
    println!("{}\n\tconnections: {}", route, con);
}


