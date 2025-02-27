use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

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
