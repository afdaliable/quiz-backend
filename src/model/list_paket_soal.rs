use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};

#[derive(Serialize, Deserialize, Clone)]
pub struct ListPaketSoal {
    pub id_nama_paket_soal: i32,
    pub nama_paket_soal: String,
    pub id_kategori_soal: i32,
    pub kategori_soal: String,
    pub jumlah_soal: i64,
}

impl<'c> FromRow<'c, MySqlRow> for ListPaketSoal {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(ListPaketSoal {
            id_nama_paket_soal: row.get("id_nama_paket_soal"),
            nama_paket_soal: row.get("nama_paket_soal"),
            id_kategori_soal: row.get("id_kategori_soal"),
            kategori_soal: row.get("kategori_soal"),
            jumlah_soal: row.get("jumlah_soal"),
        })
    }
}