use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaketSoalItemRequest {
    pub paket_soal_id: i32,
    pub soal_id: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaketSoalItemWithDetails {
    pub id: i32,
    pub paket_soal_id: i32,
    pub soal_id: i32,
    pub soal_pertanyaan: String,
    pub soal_kategori: Option<String>,
    pub soal_tingkat_kesulitan: Option<String>,
}

impl<'c> FromRow<'c, MySqlRow> for PaketSoalItemWithDetails {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(PaketSoalItemWithDetails {
            id: row.get("id"),
            paket_soal_id: row.get("paket_soal_id"),
            soal_id: row.get("soal_id"),
            soal_pertanyaan: row.get("soal_pertanyaan"),
            soal_kategori: row.get("soal_kategori"),
            soal_tingkat_kesulitan: row.get("soal_tingkat_kesulitan"),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MappingRequest {
    pub paket_soal_id: i32,
    pub soal_ids: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MappingResponse {
    pub success: bool,
    pub message: String,
    pub mapped_count: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AvailableSoal {
    pub id: i32,
    pub pertanyaan: String,
    pub opt1: String,
    pub opt2: String,
    pub opt3: String,
    pub opt4: String,
    pub opt5: String,
    pub correct_answer: String,
    pub solution: String,
    pub sumberfile: Option<String>,
    pub modul: Option<String>,
    pub pelajaran: Option<String>,
    pub tag: Option<String>,
    pub kategori: Option<String>,
    pub tingkat_kesulitan: Option<String>,
    pub is_mapped: bool,
}

impl<'c> FromRow<'c, MySqlRow> for AvailableSoal {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(AvailableSoal {
            id: row.get("id"),
            pertanyaan: row.get("pertanyaan"),
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
            kategori: row.get("kategori"),
            tingkat_kesulitan: row.get("tingkat_kesulitan"),
            is_mapped: row.get::<i32, _>("is_mapped") > 0,
        })
    }
}