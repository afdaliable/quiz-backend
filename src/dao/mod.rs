use super::model::Soal;
use super::model::KategoriSoal;
use super::model::PaketSoal;
use super::model::PaketSoalItem;
use crate::model::premium_plan::PremiumPlan;
use crate::model::user_subscription::UserSubscription;
use crate::model::premium_quiz_access::PremiumQuizAccess;
use crate::model::payment_transaction::PaymentTransaction;
use crate::model::license_code::LicenseCode;

pub mod db_context;
//use super::model::User;
// mod group_dao;
mod soal_dao;
mod paket_soal_response;
mod user_dao;
mod kategori_soal_dao;
mod session_dao;
mod quiz_session_dao;
mod premium_plan_dao;
mod user_subscription_dao;
mod premium_quiz_access_dao;
mod payment_transaction_dao;
mod license_code_dao;
pub mod license_dao;
// mod user_to_group_dao;

pub use db_context::Database;
pub type Table<'c, T> = db_context::Table<'c, T>;
pub type JoinTable<'c, T1, T2,T3,T4> = db_context::JoinTable<'c, T1, T2,T3,T4>;

use sqlx::Pool;
use sqlx::MySql;
