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
    pub opt1: Option<String>,
    pub opt2: Option<String>,
    pub opt3: Option<String>,
    pub opt4: Option<String>,
    pub opt5: Option<String>,
    pub correct_answer: Option<String>,
    pub solution: Option<String>,
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
            id: row.try_get("id")?,
            pertanyaan: row.try_get("pertanyaan")?,
            opt1: row.try_get("opt1")?,
            opt2: row.try_get("opt2")?,
            opt3: row.try_get("opt3")?,
            opt4: row.try_get("opt4")?,
            opt5: row.try_get("opt5")?,
            correct_answer: row.try_get("correct_answer")?,
            solution: row.try_get("solution")?,
            sumberfile: row.try_get("sumberfile")?,
            modul: row.try_get("modul")?,
            pelajaran: row.try_get("pelajaran")?,
            tag: row.try_get("tag")?,
            kategori: row.try_get("kategori")?,
            tingkat_kesulitan: row.try_get("tingkat_kesulitan")?,
            is_mapped: row.try_get::<i32, _>("is_mapped")? > 0,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedAvailableSoalResponse {
    pub questions: Vec<AvailableSoal>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}