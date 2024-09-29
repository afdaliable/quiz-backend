use super::model::Soal;
use super::model::KategoriSoal;
use super::model::PaketSoal;
use super::model::PaketSoalItem;

pub mod db_context;
// mod group_dao;
mod soal_dao;
mod paket_soal_response;
// mod user_to_group_dao;

pub type Database<'c> = db_context::Database<'c>;
pub type Table<'c, T> = db_context::Table<'c, T>;
pub type JoinTable<'c, T1, T2,T3,T4> = db_context::JoinTable<'c, T1, T2,T3,T4>;
