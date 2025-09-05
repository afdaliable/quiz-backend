use crate::controller::log_request;
use crate::model::users::{AdminUser, AdminUserRequest, UserStats, UserSearchRequest, PaginatedUsersResponse};
use crate::middleware::admin_middleware::AdminMiddleware;
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest, get, post, put, delete};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::{FromRow, Row};
use sqlx::mysql::MySqlRow;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSubscriptionDetails {
    pub id: u64,
    pub user_id: String,
    pub plan_id: u64,
    pub plan_name: Option<String>,
    pub price: Option<f64>,
    pub duration_days: Option<i32>,
    pub status: String,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl<'c> FromRow<'c, MySqlRow> for UserSubscriptionDetails {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(UserSubscriptionDetails {
            id: row.get("id"),
            user_id: row.get("user_id"),
            plan_id: row.get("plan_id"),
            plan_name: row.get("plan_name"),
            price: row.get("price"),
            duration_days: row.get("duration_days"),
            status: row.get("status"),
            start_date: row.get("start_date"),
            end_date: row.get("end_date"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/users")
            .wrap(AdminMiddleware::new())
            .service(get_all_users)
            .service(create_user)
            .service(get_user_stats)  // Move stats before generic /{id} route
            .service(get_user_by_id)
            .service(update_user)
            .service(delete_user)
            .service(get_user_subscriptions)
    );
}

/// Get all users with pagination, search, and filtering
#[utoipa::path(
    get,
    path = "/admin/users",
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("limit" = Option<u32>, Query, description = "Items per page (default: 20)"),
        ("search" = Option<String>, Query, description = "Search in email or display_name"),
        ("role" = Option<String>, Query, description = "Filter by role"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("has_premium" = Option<bool>, Query, description = "Filter users with premium subscription")
    ),
    responses(
        (status = 200, description = "Users retrieved successfully", body = PaginatedUsersResponse),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("")]
async fn get_all_users(
    query: web::Query<UserSearchRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/users", &data.connections);

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100); // Max 100 items per page
    let offset = (page - 1) * limit;

    // Simplified query - get all users first, then filter in application
    let total: i64 = match sqlx::query_scalar(
        "SELECT COUNT(*) FROM dbquizapp.users u WHERE u.deleted_at IS NULL"
    )
    .fetch_one(&*data.context.soal.pool)
    .await 
    {
        Ok(count) => count,
        Err(e) => {
            println!("Error counting users: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to count users".to_string(),
            });
        }
    };

    // Get users with subscription info (simplified query)
    let users = match sqlx::query_as::<_, AdminUser>(
        r#"
        SELECT 
            u.id, u.email, u.display_name, u.picture_url, u.phone_number,
            COALESCE(u.role, 'user') as role,
            COALESCE(u.status, 'active') as status,
            u.last_login, u.created_at, u.updated_at,
            CASE 
                WHEN us.status = 'active' AND (us.end_date IS NULL OR us.end_date > NOW()) 
                THEN 'active'
                ELSE 'inactive'
            END as subscription_status,
            us.end_date as subscription_end_date,
            CASE 
                WHEN u.picture_url IS NOT NULL AND (u.picture_url LIKE '%googleapis.com%' OR u.picture_url LIKE '%googleusercontent.com%') THEN 'google'
                WHEN u.picture_url IS NOT NULL AND u.picture_url LIKE '%facebook.com%' THEN 'facebook'
                WHEN u.picture_url IS NOT NULL THEN 'oauth'
                ELSE 'local'
            END as provider
        FROM dbquizapp.users u
        LEFT JOIN dbquizapp.user_subscriptions us ON u.id = us.user_id 
            AND us.status = 'active' 
            AND (us.end_date IS NULL OR us.end_date > NOW())
        WHERE u.deleted_at IS NULL
        ORDER BY u.created_at DESC
        LIMIT ? OFFSET ?
        "#
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&*data.context.soal.pool)
    .await 
    {
        Ok(users) => users,
        Err(e) => {
            println!("Error fetching users: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch users".to_string(),
            });
        }
    };

    let total_pages = (total as f64 / limit as f64).ceil() as u32;

    HttpResponse::Ok().json(PaginatedUsersResponse {
        users,
        total,
        page,
        limit,
        total_pages,
    })
}

/// Create a new user (admin only)
#[utoipa::path(
    post,
    path = "/admin/users",
    request_body = AdminUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = AdminUser),
        (status =400, description = "Invalid user data"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("")]
async fn create_user(
    user_req: web::Json<AdminUserRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/users", &data.connections);

    let user_id = Uuid::new_v4().to_string();
    let role = user_req.role.as_deref().unwrap_or("user");
    let status = user_req.status.as_deref().unwrap_or("active");

    let result = sqlx::query(
        r#"
        INSERT INTO dbquizapp.users (id, email, display_name, role, status, phone_number, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, NOW(), NOW())
        "#
    )
    .bind(&user_id)
    .bind(&user_req.email)
    .bind(&user_req.display_name)
    .bind(role)
    .bind(status)
    .bind(&user_req.phone_number)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(_) => {
            // Fetch the created user
            match get_user_by_id_internal(&data, &user_id).await {
                Ok(user) => HttpResponse::Created().json(user),
                Err(e) => {
                    println!("Error fetching created user: {:?}", e);
                    HttpResponse::InternalServerError().json(ErrorResponse {
                        error: "User created but failed to fetch details".to_string(),
                    })
                }
            }
        }
        Err(e) => {
            println!("Error creating user: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to create user".to_string(),
            })
        }
    }
}

/// Get user by ID
#[utoipa::path(
    get,
    path = "/admin/users/{id}",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User found", body = AdminUser),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/{id}")]
async fn get_user_by_id(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/users/{id}", &data.connections);

    let user_id = path.into_inner();
    
    match get_user_by_id_internal(&data, &user_id).await {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(e) => {
            println!("Error fetching user: {:?}", e);
            HttpResponse::NotFound().json(ErrorResponse {
                error: "User not found".to_string(),
            })
        }
    }
}

/// Update user
#[utoipa::path(
    put,
    path = "/admin/users/{id}",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    request_body = AdminUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = AdminUser),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[put("/{id}")]
async fn update_user(
    path: web::Path<String>,
    user_req: web::Json<AdminUserRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("PUT /admin/users/{id}", &data.connections);

    let user_id = path.into_inner();
    let role = user_req.role.as_deref().unwrap_or("user");
    let status = user_req.status.as_deref().unwrap_or("active");

    let result = sqlx::query(
        r#"
        UPDATE dbquizapp.users 
        SET email = ?, display_name = ?, role = ?, status = ?, phone_number = ?, updated_at = NOW()
        WHERE id = ? AND deleted_at IS NULL
        "#
    )
    .bind(&user_req.email)
    .bind(&user_req.display_name)
    .bind(role)
    .bind(status)
    .bind(&user_req.phone_number)
    .bind(&user_id)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                match get_user_by_id_internal(&data, &user_id).await {
                    Ok(user) => HttpResponse::Ok().json(user),
                    Err(e) => {
                        println!("Error fetching updated user: {:?}", e);
                        HttpResponse::InternalServerError().json(ErrorResponse {
                            error: "User updated but failed to fetch details".to_string(),
                        })
                    }
                }
            } else {
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "User not found".to_string(),
                })
            }
        }
        Err(e) => {
            println!("Error updating user: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to update user".to_string(),
            })
        }
    }
}

/// Delete user (soft delete)
#[utoipa::path(
    delete,
    path = "/admin/users/{id}",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 204, description = "User deleted successfully"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[delete("/{id}")]
async fn delete_user(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("DELETE /admin/users/{id}", &data.connections);

    let user_id = path.into_inner();

    let result = sqlx::query(
        "UPDATE dbquizapp.users SET deleted_at = NOW() WHERE id = ? AND deleted_at IS NULL"
    )
    .bind(&user_id)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                HttpResponse::NoContent().finish()
            } else {
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "User not found".to_string(),
                })
            }
        }
        Err(e) => {
            println!("Error deleting user: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to delete user".to_string(),
            })
        }
    }
}

/// Get user subscriptions
#[utoipa::path(
    get,
    path = "/admin/users/{id}/subscriptions",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User subscriptions retrieved successfully"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/{id}/subscriptions")]
async fn get_user_subscriptions(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/users/{id}/subscriptions", &data.connections);

    let user_id = path.into_inner();

    let result = sqlx::query_as::<_, UserSubscriptionDetails>(
        r#"
        SELECT us.*, pp.name as plan_name, pp.price, pp.duration_days
        FROM dbquizapp.user_subscriptions us
        LEFT JOIN dbquizapp.premium_plans pp ON us.plan_id = pp.id
        WHERE us.user_id = ?
        ORDER BY us.created_at DESC
        "#
    )
    .bind(&user_id)
    .fetch_all(&*data.context.user_subscriptions.pool)
    .await;

    match result {
        Ok(subscriptions) => HttpResponse::Ok().json(subscriptions),
        Err(e) => {
            println!("Error fetching user subscriptions: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch user subscriptions".to_string(),
            })
        }
    }
}

/// Get user statistics
#[utoipa::path(
    get,
    path = "/admin/users/stats",
    responses(
        (status = 200, description = "User statistics retrieved successfully", body = UserStats),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/stats")]
async fn get_user_stats(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/users/stats", &data.connections);

    let stats_query = r#"
        SELECT 
            (SELECT COUNT(*) FROM dbquizapp.users WHERE deleted_at IS NULL) as total_users,
            (SELECT COUNT(*) FROM dbquizapp.users WHERE status = 'active' AND deleted_at IS NULL) as active_users,
            (SELECT COUNT(*) FROM dbquizapp.users WHERE role IN ('admin', 'superadmin') AND deleted_at IS NULL) as admin_users,
            (SELECT COUNT(DISTINCT u.id) FROM dbquizapp.users u
             LEFT JOIN dbquizapp.user_subscriptions us ON u.id = us.user_id
             WHERE u.deleted_at IS NULL 
               AND us.status = 'active' 
               AND (us.end_date IS NULL OR us.end_date > NOW())) as users_with_premium,
            (SELECT COUNT(*) FROM dbquizapp.users WHERE DATE(created_at) = CURDATE() AND deleted_at IS NULL) as users_registered_today,
            (SELECT COUNT(*) FROM dbquizapp.users WHERE YEAR(created_at) = YEAR(NOW()) AND MONTH(created_at) = MONTH(NOW()) AND deleted_at IS NULL) as users_registered_this_month
    "#;

    let result = sqlx::query_as::<_, UserStats>(stats_query)
        .fetch_one(&*data.context.soal.pool)
        .await;

    match result {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => {
            println!("Error fetching user stats: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch user statistics".to_string(),
            })
        }
    }
}

// Helper function to get user by ID with subscription info
async fn get_user_by_id_internal(
    data: &web::Data<AppState<'_>>,
    user_id: &str,
) -> Result<AdminUser, sqlx::Error> {
    sqlx::query_as::<_, AdminUser>(
        r#"
        SELECT 
            u.id, u.email, u.display_name, u.picture_url, u.phone_number,
            COALESCE(u.role, 'user') as role,
            COALESCE(u.status, 'active') as status,
            u.last_login, u.created_at, u.updated_at,
            CASE 
                WHEN us.status = 'active' AND (us.end_date IS NULL OR us.end_date > NOW()) 
                THEN 'active'
                ELSE 'inactive'
            END as subscription_status,
            us.end_date as subscription_end_date,
            CASE 
                WHEN u.picture_url IS NOT NULL AND (u.picture_url LIKE '%googleapis.com%' OR u.picture_url LIKE '%googleusercontent.com%') THEN 'google'
                WHEN u.picture_url IS NOT NULL AND u.picture_url LIKE '%facebook.com%' THEN 'facebook'
                WHEN u.picture_url IS NOT NULL THEN 'oauth'
                ELSE 'local'
            END as provider
        FROM dbquizapp.users u
        LEFT JOIN dbquizapp.user_subscriptions us ON u.id = us.user_id 
            AND us.status = 'active' 
            AND (us.end_date IS NULL OR us.end_date > NOW())
        WHERE u.id = ? AND u.deleted_at IS NULL
        "#
    )
    .bind(user_id)
    .fetch_one(&*data.context.soal.pool)
    .await
}