use actix_web::{get, post, put, web, HttpResponse, Responder};
use serde_json::json;
use crate::AppState;
use crate::middleware::auth_middleware::AuthenticatedUser;
use crate::model::public_profile::{
    ChangeUsernameRequest, UsernameCheckResponse, UpdatePrivacyRequest,
    BadgeInfo, PublicProfileResponse, PrivacySettings,
};
use serde::Deserialize;

const RESERVED_USERNAMES: &[&str] = &[
    "admin", "administrator", "root", "superuser", "support", "help",
    "me", "profile", "user", "users", "api", "quizku", "quiz",
    "system", "official", "moderator", "mod",
];

fn validate_username(username: &str) -> Result<(), &'static str> {
    if username.len() < 3 {
        return Err("Username minimal 3 karakter");
    }
    if username.len() > 30 {
        return Err("Username maksimal 30 karakter");
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Username hanya boleh berisi huruf, angka, dan underscore");
    }
    if username.starts_with('_') || username.ends_with('_') {
        return Err("Username tidak boleh diawali atau diakhiri underscore");
    }
    if RESERVED_USERNAMES.contains(&username.to_lowercase().as_str()) {
        return Err("Username tidak tersedia");
    }
    Ok(())
}

/// Compute badges based on stats (no DB table needed)
fn compute_badges(
    total_quizzes: i64,
    best_score: i64,
    streak: i64,
    unique_categories: i64,
) -> Vec<BadgeInfo> {
    vec![
        BadgeInfo {
            id: "pemula".to_string(),
            name: "Pemula".to_string(),
            description: "Selesaikan kuis pertama".to_string(),
            icon: "🎓".to_string(),
            earned: total_quizzes >= 1,
        },
        BadgeInfo {
            id: "rajin".to_string(),
            name: "Rajin Belajar".to_string(),
            description: "Selesaikan 10 kuis".to_string(),
            icon: "📚".to_string(),
            earned: total_quizzes >= 10,
        },
        BadgeInfo {
            id: "konsisten".to_string(),
            name: "Konsisten".to_string(),
            description: "Streak 7 hari berturut-turut".to_string(),
            icon: "🔥".to_string(),
            earned: streak >= 7,
        },
        BadgeInfo {
            id: "sempurna".to_string(),
            name: "Sempurna".to_string(),
            description: "Raih skor 100 dalam satu kuis".to_string(),
            icon: "⭐".to_string(),
            earned: best_score >= 100,
        },
        BadgeInfo {
            id: "kilat".to_string(),
            name: "Kilat".to_string(),
            description: "Raih skor di atas 80".to_string(),
            icon: "⚡".to_string(),
            earned: best_score >= 80,
        },
        BadgeInfo {
            id: "all_round".to_string(),
            name: "Serba Bisa".to_string(),
            description: "Kerjakan soal dari 5 kategori berbeda".to_string(),
            icon: "🌟".to_string(),
            earned: unique_categories >= 5,
        },
    ]
}

#[derive(Deserialize)]
pub struct UsernameCheckQuery {
    pub username: String,
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_public_profile)
       .service(check_username_availability)
       .service(set_username)
       .service(update_privacy_settings);
}

/// GET /users/profile/{username} — public, no auth required
#[get("/users/profile/{username}")]
async fn get_public_profile(
    path: web::Path<String>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    let username = path.into_inner();

    let user = match app_state.context.users.get_user_by_username(&username).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().json(json!({
            "error": "not_found",
            "message": "Profil tidak ditemukan",
        })),
        Err(e) => {
            eprintln!("get_public_profile DB error: {:?}", e);
            return HttpResponse::InternalServerError().json(json!({"error": "internal_error"}));
        }
    };

    if !user.profile_public {
        return HttpResponse::NotFound().json(json!({
            "error": "not_found",
            "message": "Profil ini bersifat privat",
        }));
    }

    let privacy: PrivacySettings = user.privacy_settings
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    // Stats
    let stats = if privacy.show_stats {
        match app_state.context.users.get_public_stats(&user.id).await {
            Ok(s) => Some(s),
            Err(e) => { eprintln!("get_public_stats error: {:?}", e); None }
        }
    } else {
        None
    };

    // Badges
    let badges = if privacy.show_badges {
        let (total_quizzes, best_score, streak, unique_cats) = if let Some(ref s) = stats {
            (s.total_quizzes, s.best_score, s.learning_streak_days, 0i64)
        } else {
            match app_state.context.users.get_public_stats(&user.id).await {
                Ok(s) => (s.total_quizzes, s.best_score, s.learning_streak_days, 0i64),
                Err(_) => (0, 0, 0, 0),
            }
        };

        // Count unique categories
        let unique_categories = match sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(DISTINCT kategori_soal) FROM quiz_sessions WHERE user_id = ? AND is_completed = TRUE"
        )
        .bind(&user.id)
        .fetch_one(&*app_state.context.users.pool)
        .await {
            Ok(n) => n,
            Err(_) => unique_cats,
        };

        Some(compute_badges(total_quizzes, best_score, streak, unique_categories))
    } else {
        None
    };

    // Best scores per category
    let best_scores = if privacy.show_best_scores {
        match app_state.context.users.get_best_scores_by_category(&user.id).await {
            Ok(scores) => Some(scores),
            Err(e) => { eprintln!("get_best_scores error: {:?}", e); None }
        }
    } else {
        None
    };

    HttpResponse::Ok().json(PublicProfileResponse {
        username: user.username.unwrap_or_default(),
        display_name: user.display_name,
        picture_url: user.picture_url,
        joined_at: user.created_at,
        stats,
        badges,
        best_scores,
    })
}

/// GET /users/username/check?username=xxx — public
#[get("/users/username/check")]
async fn check_username_availability(
    query: web::Query<UsernameCheckQuery>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    let username = query.username.trim().to_lowercase();

    if let Err(msg) = validate_username(&username) {
        return HttpResponse::Ok().json(UsernameCheckResponse {
            available: false,
            message: msg.to_string(),
        });
    }

    match app_state.context.users.username_exists(&username).await {
        Ok(exists) => HttpResponse::Ok().json(UsernameCheckResponse {
            available: !exists,
            message: if exists {
                "Username sudah digunakan".to_string()
            } else {
                "Username tersedia".to_string()
            },
        }),
        Err(e) => {
            eprintln!("username_exists error: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error": "internal_error"}))
        }
    }
}

/// POST /users/me/username — set/change username (auth required)
#[post("/users/me/username")]
async fn set_username(
    auth_user: AuthenticatedUser,
    body: web::Json<ChangeUsernameRequest>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    let username = body.username.trim().to_lowercase();

    if let Err(msg) = validate_username(&username) {
        return HttpResponse::BadRequest().json(json!({
            "error": "invalid_username",
            "message": msg,
        }));
    }

    match app_state.context.users.username_exists(&username).await {
        Ok(true) => {
            // Check if it's the current user's own username
            if let Ok(Some(user)) = app_state.context.users.get_user_by_id(&auth_user.user_id).await {
                if user.username.as_deref() == Some(&username) {
                    return HttpResponse::Ok().json(json!({"message": "Username tidak berubah"}));
                }
            }
            return HttpResponse::Conflict().json(json!({
                "error": "username_taken",
                "message": "Username sudah digunakan",
            }));
        }
        Ok(false) => {}
        Err(e) => {
            eprintln!("username_exists error: {:?}", e);
            return HttpResponse::InternalServerError().json(json!({"error": "internal_error"}));
        }
    }

    match app_state.context.users.update_username(&auth_user.user_id, &username).await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "message": "Username berhasil diubah",
            "username": username,
        })),
        Err(e) => {
            eprintln!("update_username error: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error": "internal_error"}))
        }
    }
}

/// PUT /users/me/privacy — update privacy settings (auth required)
#[put("/users/me/privacy")]
async fn update_privacy_settings(
    auth_user: AuthenticatedUser,
    body: web::Json<UpdatePrivacyRequest>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    // Load current user to merge privacy settings
    let user = match app_state.context.users.get_user_by_id(&auth_user.user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "user_not_found"})),
        Err(e) => {
            eprintln!("get_user_by_id error: {:?}", e);
            return HttpResponse::InternalServerError().json(json!({"error": "internal_error"}));
        }
    };

    let mut current_privacy: PrivacySettings = user.privacy_settings
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    if let Some(show_stats) = body.show_stats { current_privacy.show_stats = show_stats; }
    if let Some(show_badges) = body.show_badges { current_privacy.show_badges = show_badges; }
    if let Some(show_best) = body.show_best_scores { current_privacy.show_best_scores = show_best; }

    let privacy_json = serde_json::to_string(&current_privacy).ok();

    match app_state.context.users.update_privacy(
        &auth_user.user_id,
        body.profile_public,
        privacy_json.as_deref(),
    ).await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "message": "Pengaturan privasi berhasil disimpan",
            "profile_public": body.profile_public.unwrap_or(user.profile_public),
            "privacy_settings": current_privacy,
        })),
        Err(e) => {
            eprintln!("update_privacy error: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error": "internal_error"}))
        }
    }
}
