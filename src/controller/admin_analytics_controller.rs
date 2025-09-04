use crate::controller::log_request;
use crate::middleware::admin_middleware::AdminMiddleware;
use crate::model::analytics::{DashboardStats, UserAnalytics, UserProviderStats, DailyGrowth, QuizSessionAnalytics, PopularPackage, PackageCompletionRate, RevenueAnalytics, RevenuePlan, MonthlyRevenue};
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest, get};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug)]
struct BasicUserStats {
    total_users: i64,
    new_users_today: Option<i64>,
    new_users_this_week: Option<i64>,
    new_users_this_month: Option<i64>,
    active_users_today: Option<i64>,
}

impl<'c> FromRow<'c, MySqlRow> for BasicUserStats {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(BasicUserStats {
            total_users: row.get("total_users"),
            new_users_today: row.get("new_users_today"),
            new_users_this_week: row.get("new_users_this_week"),
            new_users_this_month: row.get("new_users_this_month"),
            active_users_today: row.get("active_users_today"),
        })
    }
}

#[derive(Debug)]
struct BasicSessionStats {
    total_sessions: i64,
    completed_sessions: Option<i64>,
    active_sessions: Option<i64>,
    sessions_today: Option<i64>,
    sessions_this_week: Option<i64>,
    sessions_this_month: Option<i64>,
}

impl<'c> FromRow<'c, MySqlRow> for BasicSessionStats {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(BasicSessionStats {
            total_sessions: row.get("total_sessions"),
            completed_sessions: row.get("completed_sessions"),
            active_sessions: row.get("active_sessions"),
            sessions_today: row.get("sessions_today"),
            sessions_this_week: row.get("sessions_this_week"),
            sessions_this_month: row.get("sessions_this_month"),
        })
    }
}

#[derive(Debug)]
struct BasicRevenueStats {
    total_revenue: Option<f64>,
    revenue_this_month: Option<f64>,
    revenue_last_month: Option<f64>,
}

impl<'c> FromRow<'c, MySqlRow> for BasicRevenueStats {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(BasicRevenueStats {
            total_revenue: row.get("total_revenue"),
            revenue_this_month: row.get("revenue_this_month"),
            revenue_last_month: row.get("revenue_last_month"),
        })
    }
}

#[derive(Debug)]
struct SubscriptionStats {
    active_subscriptions: Option<i64>,
    expired_subscriptions: Option<i64>,
}

impl<'c> FromRow<'c, MySqlRow> for SubscriptionStats {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(SubscriptionStats {
            active_subscriptions: row.get("active_subscriptions"),
            expired_subscriptions: row.get("expired_subscriptions"),
        })
    }
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/analytics")
            .wrap(AdminMiddleware::new())
            .service(get_dashboard_stats)
            .service(get_user_analytics)
            .service(get_quiz_session_analytics)
            .service(get_revenue_analytics)
    );
}

/// Get dashboard statistics
#[utoipa::path(
    get,
    path = "/admin/analytics/dashboard",
    responses(
        (status = 200, description = "Dashboard statistics retrieved successfully", body = DashboardStats),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/dashboard")]
async fn get_dashboard_stats(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/analytics/dashboard", &data.connections);

    let query = r#"
        SELECT 
            (SELECT COUNT(*) FROM dbquizapp.users WHERE deleted_at IS NULL) as total_users,
            (SELECT COUNT(*) FROM dbquizapp.users WHERE status = 'active' AND deleted_at IS NULL) as active_users,
            (SELECT COUNT(*) FROM dbquizapp.soal) as total_questions,
            (SELECT COUNT(*) FROM dbquizapp.paket_soal) as total_packages,
            (SELECT COUNT(*) FROM dbquizapp.kategori_soal) as total_categories,
            (SELECT COUNT(*) FROM dbquizapp.quiz_sessions WHERE status = 'in_progress') as active_sessions,
            (SELECT COUNT(*) FROM dbquizapp.user_subscriptions WHERE status = 'active' AND (end_date IS NULL OR end_date > NOW())) as premium_subscriptions,
            COALESCE((SELECT SUM(pp.price) FROM dbquizapp.user_subscriptions us 
                     JOIN dbquizapp.premium_plans pp ON us.plan_id = pp.id 
                     WHERE us.status = 'active' AND YEAR(us.created_at) = YEAR(NOW()) AND MONTH(us.created_at) = MONTH(NOW())), 0) as revenue_this_month
    "#;

    match sqlx::query_as::<_, DashboardStats>(query)
        .fetch_one(&*data.context.users.pool)
        .await 
    {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => {
            println!("Error fetching dashboard stats: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch dashboard statistics".to_string(),
            })
        }
    }
}

/// Get user analytics
#[utoipa::path(
    get,
    path = "/admin/analytics/users",
    responses(
        (status = 200, description = "User analytics retrieved successfully", body = UserAnalytics),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/users")]
async fn get_user_analytics(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/analytics/users", &data.connections);

    // Get basic user stats
    let basic_stats = sqlx::query_as::<_, BasicUserStats>(
        r#"
        SELECT 
            COUNT(*) as total_users,
            (SELECT COUNT(*) FROM dbquizapp.users WHERE DATE(created_at) = CURDATE() AND deleted_at IS NULL) as new_users_today,
            (SELECT COUNT(*) FROM dbquizapp.users WHERE created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY) AND deleted_at IS NULL) as new_users_this_week,
            (SELECT COUNT(*) FROM dbquizapp.users WHERE YEAR(created_at) = YEAR(NOW()) AND MONTH(created_at) = MONTH(NOW()) AND deleted_at IS NULL) as new_users_this_month,
            (SELECT COUNT(*) FROM dbquizapp.users WHERE DATE(last_login) = CURDATE() AND deleted_at IS NULL) as active_users_today
        FROM dbquizapp.users WHERE deleted_at IS NULL
        "#
    )
    .fetch_one(&*data.context.users.pool)
    .await;

    let (total_users, new_users_today, new_users_this_week, new_users_this_month, active_users_today) = match basic_stats {
        Ok(stats) => (
            stats.total_users,
            stats.new_users_today.unwrap_or(0),
            stats.new_users_this_week.unwrap_or(0),
            stats.new_users_this_month.unwrap_or(0),
            stats.active_users_today.unwrap_or(0),
        ),
        Err(e) => {
            println!("Error fetching basic user stats: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch user statistics".to_string(),
            });
        }
    };

    // Get users by provider
    let users_by_provider = match sqlx::query_as::<_, UserProviderStats>(
        "SELECT COALESCE(provider, 'email') as provider, COUNT(*) as count FROM dbquizapp.users WHERE deleted_at IS NULL GROUP BY provider"
    )
    .fetch_all(&*data.context.users.pool)
    .await 
    {
        Ok(stats) => stats,
        Err(e) => {
            println!("Error fetching users by provider: {:?}", e);
            vec![]
        }
    };

    // Get daily growth for last 30 days
    let users_growth_chart = match sqlx::query_as::<_, DailyGrowth>(
        r#"
        SELECT DATE(created_at) as date, COUNT(*) as count 
        FROM dbquizapp.users 
        WHERE created_at >= DATE_SUB(NOW(), INTERVAL 30 DAY) AND deleted_at IS NULL
        GROUP BY DATE(created_at) 
        ORDER BY date
        "#
    )
    .fetch_all(&*data.context.users.pool)
    .await 
    {
        Ok(growth) => growth,
        Err(e) => {
            println!("Error fetching user growth chart: {:?}", e);
            vec![]
        }
    };

    let user_analytics = UserAnalytics {
        total_users,
        new_users_today,
        new_users_this_week,
        new_users_this_month,
        active_users_today,
        users_by_provider,
        users_growth_chart,
    };

    HttpResponse::Ok().json(user_analytics)
}

/// Get quiz session analytics
#[utoipa::path(
    get,
    path = "/admin/analytics/quiz-sessions",
    responses(
        (status = 200, description = "Quiz session analytics retrieved successfully", body = QuizSessionAnalytics),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/quiz-sessions")]
async fn get_quiz_session_analytics(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/analytics/quiz-sessions", &data.connections);

    // Get basic session stats
    let basic_stats = sqlx::query_as::<_, BasicSessionStats>(
        r#"
        SELECT 
            COUNT(*) as total_sessions,
            (SELECT COUNT(*) FROM dbquizapp.quiz_sessions WHERE status = 'completed') as completed_sessions,
            (SELECT COUNT(*) FROM dbquizapp.quiz_sessions WHERE status = 'in_progress') as active_sessions,
            (SELECT COUNT(*) FROM dbquizapp.quiz_sessions WHERE DATE(created_at) = CURDATE()) as sessions_today,
            (SELECT COUNT(*) FROM dbquizapp.quiz_sessions WHERE created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY)) as sessions_this_week,
            (SELECT COUNT(*) FROM dbquizapp.quiz_sessions WHERE YEAR(created_at) = YEAR(NOW()) AND MONTH(created_at) = MONTH(NOW())) as sessions_this_month
        FROM dbquizapp.quiz_sessions
        "#
    )
    .fetch_one(&*data.context.quiz_sessions.pool)
    .await;

    let (total_sessions, completed_sessions, active_sessions, sessions_today, sessions_this_week, sessions_this_month) = match basic_stats {
        Ok(stats) => (
            stats.total_sessions,
            stats.completed_sessions.unwrap_or(0),
            stats.active_sessions.unwrap_or(0),
            stats.sessions_today.unwrap_or(0),
            stats.sessions_this_week.unwrap_or(0),
            stats.sessions_this_month.unwrap_or(0),
        ),
        Err(e) => {
            println!("Error fetching basic session stats: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch session statistics".to_string(),
            });
        }
    };

    let average_completion_rate = if total_sessions > 0 {
        (completed_sessions as f64 / total_sessions as f64) * 100.0
    } else {
        0.0
    };

    // Get popular packages
    let popular_packages = match sqlx::query_as::<_, PopularPackage>(
        r#"
        SELECT 
            qs.nama_paket_soal as package_name,
            qs.kategori_soal as category_name,
            COUNT(*) as session_count
        FROM dbquizapp.quiz_sessions qs
        GROUP BY qs.nama_paket_soal, qs.kategori_soal
        ORDER BY session_count DESC
        LIMIT 10
        "#
    )
    .fetch_all(&*data.context.quiz_sessions.pool)
    .await 
    {
        Ok(packages) => packages,
        Err(e) => {
            println!("Error fetching popular packages: {:?}", e);
            vec![]
        }
    };

    // Get completion rate by package
    let completion_rate_by_package = match sqlx::query_as::<_, PackageCompletionRate>(
        r#"
        SELECT 
            qs.nama_paket_soal as package_name,
            COUNT(*) as total_sessions,
            COUNT(CASE WHEN qs.status = 'completed' THEN 1 END) as completed_sessions,
            (COUNT(CASE WHEN qs.status = 'completed' THEN 1 END) * 100.0 / COUNT(*)) as completion_rate
        FROM dbquizapp.quiz_sessions qs
        GROUP BY qs.nama_paket_soal
        HAVING COUNT(*) > 5
        ORDER BY completion_rate DESC
        LIMIT 10
        "#
    )
    .fetch_all(&*data.context.quiz_sessions.pool)
    .await 
    {
        Ok(rates) => rates,
        Err(e) => {
            println!("Error fetching completion rates: {:?}", e);
            vec![]
        }
    };

    let session_analytics = QuizSessionAnalytics {
        total_sessions,
        completed_sessions,
        active_sessions,
        average_completion_rate,
        sessions_today,
        sessions_this_week,
        sessions_this_month,
        popular_packages,
        completion_rate_by_package,
    };

    HttpResponse::Ok().json(session_analytics)
}

/// Get revenue analytics
#[utoipa::path(
    get,
    path = "/admin/analytics/revenue",
    responses(
        (status = 200, description = "Revenue analytics retrieved successfully", body = RevenueAnalytics),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/revenue")]
async fn get_revenue_analytics(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/analytics/revenue", &data.connections);

    // Get basic revenue stats
    let basic_stats = sqlx::query_as::<_, BasicRevenueStats>(
        r#"
        SELECT 
            COALESCE(SUM(pp.price), 0) as total_revenue,
            COALESCE((SELECT SUM(pp2.price) FROM dbquizapp.user_subscriptions us2 
                     JOIN dbquizapp.premium_plans pp2 ON us2.plan_id = pp2.id 
                     WHERE YEAR(us2.created_at) = YEAR(NOW()) AND MONTH(us2.created_at) = MONTH(NOW())), 0) as revenue_this_month,
            COALESCE((SELECT SUM(pp3.price) FROM dbquizapp.user_subscriptions us3 
                     JOIN dbquizapp.premium_plans pp3 ON us3.plan_id = pp3.id 
                     WHERE YEAR(us3.created_at) = YEAR(NOW()) AND MONTH(us3.created_at) = MONTH(NOW()) - 1), 0) as revenue_last_month
        FROM dbquizapp.user_subscriptions us
        JOIN dbquizapp.premium_plans pp ON us.plan_id = pp.id
        WHERE us.status = 'active'
        "#
    )
    .fetch_one(&*data.context.user_subscriptions.pool)
    .await;

    let (total_revenue, revenue_this_month, revenue_last_month) = match basic_stats {
        Ok(stats) => (
            stats.total_revenue.unwrap_or(0.0),
            stats.revenue_this_month.unwrap_or(0.0),
            stats.revenue_last_month.unwrap_or(0.0),
        ),
        Err(e) => {
            println!("Error fetching basic revenue stats: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch revenue statistics".to_string(),
            });
        }
    };

    let growth_percentage = if revenue_last_month > 0.0 {
        ((revenue_this_month - revenue_last_month) / revenue_last_month) * 100.0
    } else {
        0.0
    };

    // Get subscription counts
    let subscription_stats = sqlx::query_as::<_, SubscriptionStats>(
        r#"
        SELECT 
            (SELECT COUNT(*) FROM dbquizapp.user_subscriptions WHERE status = 'active' AND (end_date IS NULL OR end_date > NOW())) as active_subscriptions,
            (SELECT COUNT(*) FROM dbquizapp.user_subscriptions WHERE status = 'expired' OR end_date < NOW()) as expired_subscriptions
        "#
    )
    .fetch_one(&*data.context.user_subscriptions.pool)
    .await;

    let (active_subscriptions, expired_subscriptions) = match subscription_stats {
        Ok(stats) => (
            stats.active_subscriptions.unwrap_or(0),
            stats.expired_subscriptions.unwrap_or(0),
        ),
        Err(e) => {
            println!("Error fetching subscription stats: {:?}", e);
            (0, 0)
        }
    };

    // Get revenue by plan
    let revenue_by_plan = match sqlx::query_as::<_, RevenuePlan>(
        r#"
        SELECT 
            pp.name as plan_name,
            COUNT(us.id) as active_subscriptions,
            SUM(pp.price) as total_revenue
        FROM dbquizapp.premium_plans pp
        LEFT JOIN dbquizapp.user_subscriptions us ON pp.id = us.plan_id AND us.status = 'active'
        GROUP BY pp.id, pp.name
        ORDER BY total_revenue DESC
        "#
    )
    .fetch_all(&*data.context.premium_plans.pool)
    .await 
    {
        Ok(revenue) => revenue,
        Err(e) => {
            println!("Error fetching revenue by plan: {:?}", e);
            vec![]
        }
    };

    // Get monthly revenue for chart (last 12 months)
    let monthly_revenue_chart = match sqlx::query_as::<_, MonthlyRevenue>(
        r#"
        SELECT 
            DATE_FORMAT(us.created_at, '%Y-%m') as month,
            SUM(pp.price) as revenue
        FROM dbquizapp.user_subscriptions us
        JOIN dbquizapp.premium_plans pp ON us.plan_id = pp.id
        WHERE us.created_at >= DATE_SUB(NOW(), INTERVAL 12 MONTH)
        GROUP BY DATE_FORMAT(us.created_at, '%Y-%m')
        ORDER BY month
        "#
    )
    .fetch_all(&*data.context.user_subscriptions.pool)
    .await 
    {
        Ok(chart) => chart,
        Err(e) => {
            println!("Error fetching monthly revenue chart: {:?}", e);
            vec![]
        }
    };

    let revenue_analytics = RevenueAnalytics {
        total_revenue,
        revenue_this_month,
        revenue_last_month,
        growth_percentage,
        active_subscriptions,
        expired_subscriptions,
        revenue_by_plan,
        monthly_revenue_chart,
    };

    HttpResponse::Ok().json(revenue_analytics)
}