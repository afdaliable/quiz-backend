use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

/// Dashboard statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DashboardStats {
    pub total_users: i64,
    pub active_users: i64,
    pub total_questions: i64,
    pub total_packages: i64,
    pub total_categories: i64,
    pub active_sessions: i64,
    pub premium_subscriptions: i64,
    pub revenue_this_month: f64,
}

impl<'c> FromRow<'c, MySqlRow> for DashboardStats {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(DashboardStats {
            total_users: row.get("total_users"),
            active_users: row.get("active_users"),
            total_questions: row.get("total_questions"),
            total_packages: row.get("total_packages"),
            total_categories: row.get("total_categories"),
            active_sessions: row.get("active_sessions"),
            premium_subscriptions: row.get("premium_subscriptions"),
            revenue_this_month: row.get("revenue_this_month"),
        })
    }
}

/// User analytics data
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserAnalytics {
    pub total_users: i64,
    pub new_users_today: i64,
    pub new_users_this_week: i64,
    pub new_users_this_month: i64,
    pub active_users_today: i64,
    pub users_by_provider: Vec<UserProviderStats>,
    pub users_growth_chart: Vec<DailyGrowth>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserProviderStats {
    pub provider: String,
    pub count: i64,
}

impl<'c> FromRow<'c, MySqlRow> for UserProviderStats {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(UserProviderStats {
            provider: row.get("provider"),
            count: row.get("count"),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DailyGrowth {
    pub date: String,
    pub count: i64,
}

impl<'c> FromRow<'c, MySqlRow> for DailyGrowth {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(DailyGrowth {
            date: row.get("date"),
            count: row.get("count"),
        })
    }
}

/// Quiz session analytics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QuizSessionAnalytics {
    pub total_sessions: i64,
    pub completed_sessions: i64,
    pub active_sessions: i64,
    pub average_completion_rate: f64,
    pub sessions_today: i64,
    pub sessions_this_week: i64,
    pub sessions_this_month: i64,
    pub popular_packages: Vec<PopularPackage>,
    pub completion_rate_by_package: Vec<PackageCompletionRate>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PopularPackage {
    pub package_name: String,
    pub category_name: String,
    pub session_count: i64,
}

impl<'c> FromRow<'c, MySqlRow> for PopularPackage {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(PopularPackage {
            package_name: row.get("package_name"),
            category_name: row.get("category_name"),
            session_count: row.get("session_count"),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PackageCompletionRate {
    pub package_name: String,
    pub total_sessions: i64,
    pub completed_sessions: i64,
    pub completion_rate: f64,
}

impl<'c> FromRow<'c, MySqlRow> for PackageCompletionRate {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(PackageCompletionRate {
            package_name: row.get("package_name"),
            total_sessions: row.get("total_sessions"),
            completed_sessions: row.get("completed_sessions"),
            completion_rate: row.get("completion_rate"),
        })
    }
}

/// Revenue analytics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RevenueAnalytics {
    pub total_revenue: f64,
    pub revenue_this_month: f64,
    pub revenue_last_month: f64,
    pub growth_percentage: f64,
    pub active_subscriptions: i64,
    pub expired_subscriptions: i64,
    pub revenue_by_plan: Vec<RevenuePlan>,
    pub monthly_revenue_chart: Vec<MonthlyRevenue>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RevenuePlan {
    pub plan_name: String,
    pub active_subscriptions: i64,
    pub total_revenue: f64,
}

impl<'c> FromRow<'c, MySqlRow> for RevenuePlan {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(RevenuePlan {
            plan_name: row.get("plan_name"),
            active_subscriptions: row.get("active_subscriptions"),
            total_revenue: row.get("total_revenue"),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MonthlyRevenue {
    pub month: String,
    pub revenue: f64,
}

impl<'c> FromRow<'c, MySqlRow> for MonthlyRevenue {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(MonthlyRevenue {
            month: row.get("month"),
            revenue: row.get("revenue"),
        })
    }
}