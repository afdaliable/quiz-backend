use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PremiumPlan {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub duration_days: i32,
    pub is_lifetime: bool,
    pub features: String, // JSON array of features
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PremiumPlanResponse {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub duration_days: i32,
    pub is_lifetime: bool,
    pub features: Vec<String>,
}

impl From<PremiumPlan> for PremiumPlanResponse {
    fn from(plan: PremiumPlan) -> Self {
        let features: Vec<String> = serde_json::from_str(&plan.features).unwrap_or_default();
        
        PremiumPlanResponse {
            id: plan.id,
            name: plan.name,
            description: plan.description,
            price: plan.price,
            duration_days: plan.duration_days,
            is_lifetime: plan.is_lifetime,
            features,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePremiumPlanRequest {
    pub name: String,
    pub description: String,
    pub price: f64,
    pub duration_days: i32,
    pub is_lifetime: bool,
    pub features: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePremiumPlanRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<f64>,
    pub duration_days: Option<i32>,
    pub is_lifetime: Option<bool>,
    pub features: Option<Vec<String>>,
} 