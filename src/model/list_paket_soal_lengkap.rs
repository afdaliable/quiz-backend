use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct ListPaketSoalLengkap {
    pub id_nama_paket_soal: i32,
    pub nama_paket_soal: String,
    pub id_kategori_soal: i32,
    pub kategori_soal: String,
    pub jumlah_soal: i64,
    pub koin: i32,
    pub harga: i32,
    pub is_free: bool,
    pub is_premium: bool,
}

impl<'c> FromRow<'c, MySqlRow> for ListPaketSoalLengkap {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(ListPaketSoalLengkap {
            id_nama_paket_soal: row.get("id_nama_paket_soal"),
            nama_paket_soal: row.get("nama_paket_soal"),
            id_kategori_soal: row.get("id_kategori_soal"),
            kategori_soal: row.get("kategori_soal"),
            jumlah_soal: row.get("jumlah_soal"),
            koin: row.get("koin"),
            harga: row.get("harga"),
            is_free: row.get("is_free"),
            is_premium: row.get("is_premium"),
        })
    }
}