use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use super::Soal;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct PaketSoalResponse {
    pub kategori_id: i32,
    pub nama_kategori: String,
    pub paket_soal_id: i32,
    pub nama_paket_soal: String,
    pub is_premium: bool,
    pub kumpulan_soal: Vec<Soal>
}



impl<'c> FromRow<'c, MySqlRow> for PaketSoalResponse {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(PaketSoalResponse {
            kategori_id: row.get("kategori_id"),
            nama_kategori: row.get("nama_kategori"),
            paket_soal_id: row.get("paket_soal_id"),
            nama_paket_soal: row.get("nama_paket_soal"),
            is_premium: row.get("is_premium"),
            kumpulan_soal: vec![Soal {
                id: row.get("soal_id"),
                soal: row.get("soal"),
                question_type: row.try_get("question_type").unwrap_or_else(|_| "multiple_choice".to_string()),
                opt1: row.get("opt1"),
                opt2: row.get("opt2"),
                opt3: row.get("opt3"),
                opt4: row.get("opt4"),
                opt5: row.get("opt5"),
                correct_answer: row.get("correct_answer"),
                solution: row.get("solution"),
                sumberfile: row.get("sumberfile"),
                modul: row.get("modul"),
                pelajaran: row.get("pelajaran"),    
                tag: row.get("tag"),
            }],
        })
    }
}