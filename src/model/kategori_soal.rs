use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct KategoriSoal {
    /// Category ID
    pub id: i32,
    /// Category name
    pub nama_kategori: String,
}

impl<'c> FromRow<'c, MySqlRow> for KategoriSoal {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(KategoriSoal {
            id: row.get("id"),
            nama_kategori: row.get("nama_kategori"),
        })
    }
}

// Extended category model for admin operations with additional info
#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct AdminKategoriSoal {
    /// Category ID
    pub id: i32,
    /// Category name
    pub nama_kategori: String,
    /// Number of quiz packages in this category
    pub quiz_packages_count: i64,
    /// Number of questions in this category
    pub questions_count: i64,
}

impl<'c> FromRow<'c, MySqlRow> for AdminKategoriSoal {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(AdminKategoriSoal {
            id: row.get("id"),
            nama_kategori: row.get("nama_kategori"),
            quiz_packages_count: row.get("quiz_packages_count"),
            questions_count: row.get("questions_count"),
        })
    }
}

// Request for creating/updating categories
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AdminKategoriRequest {
    /// Category name
    pub nama_kategori: String,
}
