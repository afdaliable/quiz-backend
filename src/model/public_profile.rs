use serde::{Deserialize, Serialize};
use chrono::DateTime;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrivacySettings {
    pub show_stats: bool,
    pub show_badges: bool,
    pub show_best_scores: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        PrivacySettings {
            show_stats: true,
            show_badges: true,
            show_best_scores: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicStats {
    pub total_quizzes: i64,
    pub avg_score: f64,
    pub best_score: i64,
    pub learning_streak_days: i64,
    pub favorite_category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BadgeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub earned: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryBestScore {
    pub category: String,
    pub best_score: i64,
    pub total_attempts: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicProfileResponse {
    pub username: String,
    pub display_name: String,
    pub picture_url: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub stats: Option<PublicStats>,
    pub badges: Option<Vec<BadgeInfo>>,
    pub best_scores: Option<Vec<CategoryBestScore>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeUsernameRequest {
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsernameCheckResponse {
    pub available: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePrivacyRequest {
    pub profile_public: Option<bool>,
    pub show_stats: Option<bool>,
    pub show_badges: Option<bool>,
    pub show_best_scores: Option<bool>,
}
