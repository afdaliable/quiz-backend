use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Serialize, Deserialize)]
pub struct SoalSearchResult {
    pub id: i64,
    pub soal: String,
    pub question_type: String,
    pub modul: Option<String>,
    pub pelajaran: Option<String>,
    pub tag: Option<String>,
    pub nama_paket_soal: Option<String>,
    pub kategori_soal: Option<String>,
}

impl<'r> FromRow<'r, MySqlRow> for SoalSearchResult {
    fn from_row(row: &'r MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(SoalSearchResult {
            id: row.get("id"),
            soal: row.get("soal"),
            question_type: row.get("question_type"),
            modul: row.try_get("modul").ok().flatten(),
            pelajaran: row.try_get("pelajaran").ok().flatten(),
            tag: row.try_get("tag").ok().flatten(),
            nama_paket_soal: row.try_get("nama_paket_soal").ok().flatten(),
            kategori_soal: row.try_get("kategori_soal").ok().flatten(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoalSearchResponse {
    pub results: Vec<SoalSearchResult>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
pub struct SoalSearchQuery {
    pub q: Option<String>,
    pub modul: Option<String>,
    pub pelajaran: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SearchFiltersResponse {
    pub modul: Vec<String>,
    pub pelajaran: Vec<String>,
}
