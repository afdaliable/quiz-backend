use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};

#[derive(Serialize, Deserialize, Clone)]
pub struct PaketSoalItem {
    pub id: i32,
    pub paket_soal_id: i32,
    pub soal_id: i32,
}

impl<'c> FromRow<'c, MySqlRow> for PaketSoalItem {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(PaketSoalItem {
            id: row.get("id"),
            paket_soal_id: row.get("paket_soal_id"),
            soal_id: row.get("soal_id"),
        })
    }
}
