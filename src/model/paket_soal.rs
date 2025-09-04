use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct PaketSoal {
    pub id: i32,
    pub nama_paket_soal: String,
    pub kategori_id: Option<i32>,
    pub kategori_nama: Option<String>,
    pub is_premium: bool,
    pub jumlah_soal: Option<i64>,
}

impl<'c> FromRow<'c, MySqlRow> for PaketSoal {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(PaketSoal {
            id: row.get("id"),
            nama_paket_soal: row.get("nama_paket_soal"),
            kategori_id: row.get("kategori_id"),
            kategori_nama: row.try_get("kategori_nama").ok(),
            is_premium: row.get("is_premium"),
            jumlah_soal: row.try_get("jumlah_soal").ok(),
        })
    }
}

/// Extended package model for admin operations
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AdminPaketSoal {
    pub id: i32,
    pub nama_paket_soal: String,
    pub kategori_id: Option<i32>,
    pub kategori_name: Option<String>,
    pub is_premium: bool,
    pub questions_count: i64,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl<'c> FromRow<'c, MySqlRow> for AdminPaketSoal {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(AdminPaketSoal {
            id: row.get("id"),
            nama_paket_soal: row.get("nama_paket_soal"),
            kategori_id: row.try_get("kategori_id").ok(),
            kategori_name: row.try_get("kategori_name").ok(),
            is_premium: row.get("is_premium"),
            questions_count: row.get("questions_count"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

/// Request for creating/updating packages
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AdminPaketSoalRequest {
    pub nama_paket_soal: String,
    pub kategori_id: i32,
    pub is_premium: bool,
}

/// Request for package search/filtering
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PackageSearchRequest {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub search: Option<String>,
    pub kategori_id: Option<i32>,
    pub is_premium: Option<bool>,
}

/// Paginated packages response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedPackagesResponse {
    pub packages: Vec<AdminPaketSoal>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

/// Request for adding questions to package
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddQuestionsRequest {
    pub question_ids: Vec<i32>,
}

/// Response for package operations
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PackageOperationResponse {
    pub success: bool,
    pub message: String,
    pub affected_count: Option<i32>,
}

/// Request for creating a package
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePaketSoalRequest {
    pub nama_paket_soal: String,
    pub kategori_id: Option<i32>,
    pub is_premium: bool,
}

/// Request for updating a package
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdatePaketSoalRequest {
    pub nama_paket_soal: String,
    pub kategori_id: Option<i32>,
    pub is_premium: bool,
}