use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ExamTrack {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub icon: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub question_count: i64,
}

impl<'c> FromRow<'c, MySqlRow> for ExamTrack {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(ExamTrack {
            id: row.get("id"),
            slug: row.get("slug"),
            name: row.get("name"),
            icon: row.try_get("icon").unwrap_or(None),
            status: row.get("status"),
            sort_order: row.try_get("sort_order").unwrap_or(0),
            question_count: row.try_get("question_count").unwrap_or(0),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Category {
    pub id: String,
    pub track_id: String,
    pub slug: String,
    pub name: String,
    pub sort_order: i32,
    pub question_count: i64,
}

impl<'c> FromRow<'c, MySqlRow> for Category {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Category {
            id: row.get("id"),
            track_id: row.get("track_id"),
            slug: row.get("slug"),
            name: row.get("name"),
            sort_order: row.try_get("sort_order").unwrap_or(0),
            question_count: row.try_get("question_count").unwrap_or(0),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Subcategory {
    pub id: String,
    pub category_id: String,
    pub slug: String,
    pub name: String,
    pub sort_order: i32,
    pub question_count: i64,
}

impl<'c> FromRow<'c, MySqlRow> for Subcategory {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Subcategory {
            id: row.get("id"),
            category_id: row.get("category_id"),
            slug: row.get("slug"),
            name: row.get("name"),
            sort_order: row.try_get("sort_order").unwrap_or(0),
            question_count: row.try_get("question_count").unwrap_or(0),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Topic {
    pub id: String,
    pub subcategory_id: String,
    pub slug: String,
    pub name: String,
    pub sort_order: i32,
    pub question_count: i64,
}

impl<'c> FromRow<'c, MySqlRow> for Topic {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Topic {
            id: row.get("id"),
            slug: row.get("slug"),
            subcategory_id: row.get("subcategory_id"),
            name: row.get("name"),
            sort_order: row.try_get("sort_order").unwrap_or(0),
            question_count: row.try_get("question_count").unwrap_or(0),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Tag {
    pub id: String,
    pub slug: String,
    pub label: String,
}

impl<'c> FromRow<'c, MySqlRow> for Tag {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Tag {
            id: row.get("id"),
            slug: row.get("slug"),
            label: row.get("label"),
        })
    }
}

// ─── Tree structures ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TopicWithTags {
    #[serde(flatten)]
    pub topic: Topic,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SubcategoryWithChildren {
    #[serde(flatten)]
    pub subcategory: Subcategory,
    pub topics: Vec<Topic>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CategoryWithChildren {
    #[serde(flatten)]
    pub category: Category,
    pub subcategories: Vec<SubcategoryWithChildren>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TrackWithChildren {
    #[serde(flatten)]
    pub track: ExamTrack,
    pub categories: Vec<CategoryWithChildren>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TaxonomyTree {
    pub tracks: Vec<TrackWithChildren>,
}

// ─── Context struct used by SoalWithTaxonomy ─────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TaxonomyContext {
    pub track: Option<ExamTrack>,
    pub category: Option<Category>,
    pub subcategory: Option<Subcategory>,
    pub topic: Option<Topic>,
    pub tags: Vec<Tag>,
}
