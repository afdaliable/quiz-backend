use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct HargaPaket {
    pub id: i32,
    pub id_paket_soal: i32,
    pub koin: i32,
    pub harga: f64,
    pub is_free: bool,
}

impl<'c> FromRow<'c, MySqlRow> for HargaPaket {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(HargaPaket {
            id: row.get("id"),
            id_paket_soal: row.get("id_paket_soal"),
            koin: row.get("koin"),
            harga: row.get("harga"),
            is_free: row.get("is_free"),
        })
    }
}