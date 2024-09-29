use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};

#[derive(Serialize, Deserialize, Clone)]
pub struct PaketSoal {
    pub id: i32,
    pub nama_paket_soal: String,
    pub kategori_id: i32,
}

impl<'c> FromRow<'c, MySqlRow> for PaketSoal {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(PaketSoal {
            id: row.get("id"),
            nama_paket_soal: row.get("nama_paket_soal"),
            kategori_id: row.get("kategori_id"),
        })
    }
}