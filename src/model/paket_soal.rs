use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

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

/// Difficulty distribution for generated packages
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct DifficultyMix {
    /// Number of easy questions (default: 0)
    pub easy: Option<u32>,
    /// Number of medium questions (default: 0)
    pub medium: Option<u32>,
    /// Number of hard questions (default: 0)
    pub hard: Option<u32>,
}

impl DifficultyMix {
    pub fn total(&self) -> u32 {
        self.easy.unwrap_or(0) + self.medium.unwrap_or(0) + self.hard.unwrap_or(0)
    }
}

/// Request for generating a random package
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GeneratePackageRequest {
    /// Package name
    pub nama_paket_soal: String,
    /// Filter by exam track slug (e.g. "skd", "snbt")
    pub track_slug: Option<String>,
    /// Filter by category slug (optional, narrows within track)
    pub category_slug: Option<String>,
    /// Filter by subcategory slug (optional)
    pub subcategory_slug: Option<String>,
    /// Distribution of questions per difficulty level
    pub difficulty_mix: DifficultyMix,
    /// Duration in minutes (default: 90)
    pub duration_min: Option<i32>,
    /// If true, pick questions proportionally from each source
    pub source_balance: Option<bool>,
    /// Restrict to specific sources only (empty = all sources)
    pub allowed_sources: Option<Vec<String>>,
    /// Prefix for auto-generated package code, e.g. "SKD" → "SKD-2026-001"
    pub kode_prefix: Option<String>,
}

/// Response for generate package endpoint
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GeneratePackageResponse {
    pub paket_soal_id: i32,
    pub nama_paket_soal: String,
    pub kode_paket: Option<String>,
    pub total_questions: u32,
    pub difficulty_mix: DifficultyMix,
    pub selected_question_ids: Vec<i32>,
}

/// Request for previewing source distribution without generating
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct PreviewDistributionRequest {
    pub track_slug: Option<String>,
    pub category_slug: Option<String>,
    pub subcategory_slug: Option<String>,
    pub difficulty_mix: DifficultyMix,
    pub source_balance: Option<bool>,
    pub allowed_sources: Option<Vec<String>>,
    pub kode_prefix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SourceDistributionItem {
    pub source: String,
    pub available: usize,
    pub would_pick: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PreviewDistributionResponse {
    pub total_available: usize,
    pub kode_preview: Option<String>,
    pub sources: Vec<SourceDistributionItem>,
}

// ── AFD-244: Coverage & Source Distribution ──

/// Single package entry in soal packages list
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SoalPackageItem {
    pub paket_soal_id: i32,
    pub nama_paket_soal: String,
    pub kode_paket: Option<String>,
    pub is_premium: bool,
}

impl<'c> FromRow<'c, MySqlRow> for SoalPackageItem {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(SoalPackageItem {
            paket_soal_id: row.get("paket_soal_id"),
            nama_paket_soal: row.get("nama_paket_soal"),
            kode_paket: row.try_get("kode_paket").ok(),
            is_premium: row.get("is_premium"),
        })
    }
}

/// Response for GET /admin/soal/{id}/packages
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SoalPackagesResponse {
    pub soal_id: i32,
    pub packages: Vec<SoalPackageItem>,
    pub total: usize,
}

/// Source breakdown entry within a package
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SourceDistributionEntry {
    pub source: Option<String>,
    pub count: i64,
    pub percent: f64,
}

/// Response for GET /admin/packages/{id}/source-distribution
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PackageSourceDistributionResponse {
    pub paket_soal_id: i32,
    pub nama_paket_soal: String,
    pub total_soal: i64,
    pub sources: Vec<SourceDistributionEntry>,
}

/// Single row in the source-distribution-summary list
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PackageSourceSummaryItem {
    pub paket_soal_id: i32,
    pub nama_paket_soal: String,
    pub kode_paket: Option<String>,
    pub total_soal: i64,
    pub dominant_source: Option<String>,
    pub dominant_persen: Option<f64>,
}

impl<'c> FromRow<'c, MySqlRow> for PackageSourceSummaryItem {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(PackageSourceSummaryItem {
            paket_soal_id: row.get("id"),
            nama_paket_soal: row.get("nama_paket_soal"),
            kode_paket: row.try_get("kode_paket").ok(),
            total_soal: row.get("total_soal"),
            dominant_source: row.try_get("dominant_source").ok(),
            dominant_persen: row.try_get("dominant_persen").ok(),
        })
    }
}

/// Response for GET /admin/packages/source-distribution-summary
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PackageSourceSummaryResponse {
    pub packages: Vec<PackageSourceSummaryItem>,
    pub total: usize,
}

/// Response for GET /admin/soal/coverage-stats
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SoalCoverageStats {
    pub total_soal: i64,
    pub not_in_any_package: i64,
    pub in_exactly_one_package: i64,
    pub in_multiple_packages: i64,
    pub max_package_count: i64,
    pub avg_package_count: f64,
}

// ── Simulasi Templates ──

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SimulasiTemplateSection {
    pub name: String,
    pub subcategory_slug: String,
    pub count: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SimulasiTemplate {
    pub id: i32,
    pub exam_type: String,
    pub kode_prefix: String,
    pub name: String,
    pub description: Option<String>,
    pub sections: Vec<SimulasiTemplateSection>,
    pub duration_minutes: i32,
    pub passing_score: i32,
    pub is_active: bool,
}

impl<'c> FromRow<'c, MySqlRow> for SimulasiTemplate {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        let sections_json: String = row.try_get("sections").unwrap_or_else(|_| "[]".to_string());
        let sections: Vec<SimulasiTemplateSection> =
            serde_json::from_str(&sections_json).unwrap_or_default();
        Ok(SimulasiTemplate {
            id: row.get("id"),
            exam_type: row.get("exam_type"),
            kode_prefix: row.get("kode_prefix"),
            name: row.get("name"),
            description: row.try_get("description").ok(),
            sections,
            duration_minutes: row.get("duration_minutes"),
            passing_score: row.get("passing_score"),
            is_active: row.get("is_active"),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SimulasiTemplateRequest {
    pub exam_type: String,
    pub kode_prefix: String,
    pub name: String,
    pub description: Option<String>,
    pub sections: Vec<SimulasiTemplateSection>,
    pub duration_minutes: i32,
    pub passing_score: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GenerateSimulasiRequest {
    pub exam_type: String,
    /// Max 20 per request
    pub jumlah_paket: u32,
    /// Display name prefix: "{nama_prefix} #1", "#2", ...
    pub nama_prefix: String,
    pub source_balance: Option<bool>,
    pub is_premium: Option<bool>,
    /// Auto-create exam_simulations records (default: true)
    pub create_exam_simulasi: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GeneratedSimulasiItem {
    pub paket_soal_id: i32,
    pub exam_simulasi_id: Option<i32>,
    pub kode_paket: String,
    pub nama: String,
    pub total_soal: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GenerateSimulasiResponse {
    pub generated: u32,
    pub packages: Vec<GeneratedSimulasiItem>,
    pub source_distribution_summary: HashMap<String, String>,
}
