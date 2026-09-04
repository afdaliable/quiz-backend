use actix_web::{web, HttpResponse, Result, HttpRequest};
use sqlx::{MySql, Pool, FromRow};
use crate::AppState;
use crate::controller::admin_soal_controller::build_fulltext_boolean_query;
use crate::model::paket_soal_items::{
    PaketSoalItem, PaketSoalItemRequest, PaketSoalItemWithDetails,
    MappingRequest, MappingResponse, AvailableSoal, PaginatedAvailableSoalResponse,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/paket-soal")
            .route("/{paket_soal_id}/questions", web::get().to(get_package_questions))
            .route("/{paket_soal_id}/available-questions", web::get().to(get_available_questions))
            .route("/{paket_soal_id}/map-questions", web::post().to(map_questions_to_package))
            .route("/{paket_soal_id}/unmap-questions", web::delete().to(unmap_questions_from_package))
    )
    .service(
        web::scope("/admin/paket-soal-items")
            .route("/{id}", web::delete().to(delete_mapping))
    );
}

/// Get all questions mapped to a package
#[utoipa::path(
    get,
    path = "/admin/paket-soal/{paket_soal_id}/questions",
    params(
        ("paket_soal_id" = i32, Path, description = "Package ID")
    ),
    responses(
        (status = 200, description = "List of mapped questions", body = Vec<PaketSoalItemWithDetails>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Package Questions Mapping",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_package_questions(
    data: web::Data<AppState<'_>>,
    paket_soal_id: web::Path<i32>,
    _http_req: HttpRequest,
) -> Result<HttpResponse> {
    let paket_soal_id = paket_soal_id.into_inner();
    
    let query = "
        SELECT 
            psi.id,
            psi.paket_soal_id,
            psi.soal_id,
            s.soal as soal_pertanyaan,
            ks.nama_kategori as soal_kategori,
            NULL as soal_tingkat_kesulitan
        FROM dbquizapp.paket_soal_items psi
        JOIN dbquizapp.soal s ON psi.soal_id = s.id
        JOIN dbquizapp.paket_soal ps ON psi.paket_soal_id = ps.id
        LEFT JOIN dbquizapp.kategori_soal ks ON ps.kategori_id = ks.id
        WHERE psi.paket_soal_id = ?
        ORDER BY psi.id ASC
    ";
    
    match sqlx::query(query)
        .bind(paket_soal_id)
        .fetch_all(&*data.context.soal.pool)
        .await
    {
        Ok(rows) => {
            let items: Result<Vec<PaketSoalItemWithDetails>, sqlx::Error> = rows
                .iter()
                .map(PaketSoalItemWithDetails::from_row)
                .collect();

            match items {
                Ok(items) => Ok(HttpResponse::Ok().json(items)),
                Err(err) => {
                    eprintln!("Error parsing package questions: {}", err);
                    Ok(HttpResponse::InternalServerError().json("Failed to parse package questions"))
                }
            }
        }
        Err(err) => {
            eprintln!("Error fetching package questions: {}", err);
            Ok(HttpResponse::InternalServerError().json("Failed to fetch package questions"))
        }
    }
}

#[derive(serde::Deserialize)]
pub struct AvailableQuestionsQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub search: Option<String>,
    pub modul: Option<String>,
    pub pelajaran: Option<String>,
    pub tag: Option<String>,
    /// "mapped" | "unmapped" -- absent/anything else means both.
    pub mapped: Option<String>,
}

/// Get available questions for mapping (paginated + filterable).
///
/// Was `SELECT * FROM soal` with no LIMIT at all -- confirmed live: 104MB /
/// 33.7s response, on every page load of both the standalone Mapping page
/// and the per-package mapping tab. Same fix pattern as Menu-Soal/
/// Package-Builder: server pagination + FULLTEXT search instead of loading
/// the whole table into the browser.
#[utoipa::path(
    get,
    path = "/admin/paket-soal/{paket_soal_id}/available-questions",
    params(
        ("paket_soal_id" = i32, Path, description = "Package ID"),
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("limit" = Option<u32>, Query, description = "Items per page (default: 50, max: 200)"),
        ("search" = Option<String>, Query, description = "Search in question text"),
        ("modul" = Option<String>, Query, description = "Filter by module"),
        ("pelajaran" = Option<String>, Query, description = "Filter by subject"),
        ("tag" = Option<String>, Query, description = "Filter by tag"),
        ("mapped" = Option<String>, Query, description = "'mapped' | 'unmapped' -- default both")
    ),
    responses(
        (status = 200, description = "Paginated available questions", body = PaginatedAvailableSoalResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Package Questions Mapping",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_available_questions(
    data: web::Data<AppState<'_>>,
    paket_soal_id: web::Path<i32>,
    query: web::Query<AvailableQuestionsQuery>,
    _http_req: HttpRequest,
) -> Result<HttpResponse> {
    let paket_soal_id = paket_soal_id.into_inner();
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = (page - 1) * limit;

    let mut where_conditions: Vec<String> = vec!["1=1".to_string()];
    let mut binds: Vec<String> = Vec::new();

    if let Some(search) = query.search.as_deref().filter(|s| !s.is_empty()) {
        if let Some(boolean_query) = build_fulltext_boolean_query(search) {
            where_conditions.push("MATCH(s.soal) AGAINST (? IN BOOLEAN MODE)".to_string());
            binds.push(boolean_query);
        }
    }
    if let Some(modul) = query.modul.as_deref().filter(|s| !s.is_empty()) {
        where_conditions.push("s.modul = ?".to_string());
        binds.push(modul.to_string());
    }
    if let Some(pelajaran) = query.pelajaran.as_deref().filter(|s| !s.is_empty()) {
        where_conditions.push("s.pelajaran = ?".to_string());
        binds.push(pelajaran.to_string());
    }
    if let Some(tag) = query.tag.as_deref().filter(|s| !s.is_empty()) {
        where_conditions.push("s.tag = ?".to_string());
        binds.push(tag.to_string());
    }
    match query.mapped.as_deref() {
        Some("mapped") => where_conditions.push("psi.soal_id IS NOT NULL".to_string()),
        Some("unmapped") => where_conditions.push("psi.soal_id IS NULL".to_string()),
        _ => {}
    }

    let where_clause = where_conditions.join(" AND ");
    let pool = &*data.context.soal.pool;

    let count_sql = format!(
        "SELECT COUNT(*) FROM dbquizapp.soal s \
         LEFT JOIN dbquizapp.paket_soal_items psi ON s.id = psi.soal_id AND psi.paket_soal_id = ? \
         WHERE {}",
        where_clause
    );
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql).bind(paket_soal_id);
    for b in &binds {
        count_q = count_q.bind(b);
    }
    let total = match count_q.fetch_one(pool).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error counting available questions: {}", e);
            return Ok(HttpResponse::InternalServerError().json("Failed to count available questions"));
        }
    };

    let list_sql = format!(
        "SELECT
            s.id,
            s.soal as pertanyaan,
            s.opt1,
            s.opt2,
            s.opt3,
            s.opt4,
            s.opt5,
            s.correct_answer,
            s.solution,
            s.sumberfile,
            s.modul,
            s.pelajaran,
            s.tag,
            NULL as kategori,
            NULL as tingkat_kesulitan,
            CASE WHEN psi.soal_id IS NOT NULL THEN 1 ELSE 0 END as is_mapped
        FROM dbquizapp.soal s
        LEFT JOIN dbquizapp.paket_soal_items psi ON s.id = psi.soal_id AND psi.paket_soal_id = ?
        WHERE {}
        ORDER BY s.id DESC
        LIMIT ? OFFSET ?",
        where_clause
    );
    let mut list_q = sqlx::query(&list_sql).bind(paket_soal_id);
    for b in &binds {
        list_q = list_q.bind(b);
    }
    let rows = match list_q.bind(limit).bind(offset).fetch_all(pool).await {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching available questions: {}", e);
            return Ok(HttpResponse::InternalServerError().json("Failed to fetch available questions"));
        }
    };

    let questions: Result<Vec<AvailableSoal>, sqlx::Error> =
        rows.iter().map(AvailableSoal::from_row).collect();

    match questions {
        Ok(questions) => {
            let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;
            Ok(HttpResponse::Ok().json(PaginatedAvailableSoalResponse {
                questions,
                total,
                page,
                limit,
                total_pages,
            }))
        }
        Err(e) => {
            eprintln!("Error parsing available questions: {}", e);
            Ok(HttpResponse::InternalServerError().json("Failed to parse available questions"))
        }
    }
}

/// Add questions to package mapping
#[utoipa::path(
    post,
    path = "/admin/paket-soal/{paket_soal_id}/map-questions",
    params(
        ("paket_soal_id" = i32, Path, description = "Package ID")
    ),
    request_body = MappingRequest,
    responses(
        (status = 200, description = "Questions mapped successfully", body = MappingResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Package Questions Mapping",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn map_questions_to_package(
    data: web::Data<AppState<'_>>,
    paket_soal_id: web::Path<i32>,
    request: web::Json<MappingRequest>,
    _http_req: HttpRequest,
) -> Result<HttpResponse> {
    let paket_soal_id = paket_soal_id.into_inner();
    let soal_ids = &request.soal_ids;
    
    if soal_ids.is_empty() {
        return Ok(HttpResponse::BadRequest().json("No questions provided"));
    }
    
    // Start transaction
    let mut tx = match data.context.soal.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => {
            eprintln!("Failed to begin transaction: {}", err);
            return Ok(HttpResponse::InternalServerError().json("Database transaction error"));
        }
    };
    
    let mut mapped_count = 0;
    
    for soal_id in soal_ids {
        // Check if mapping already exists
        let check_query = "SELECT id FROM dbquizapp.paket_soal_items WHERE paket_soal_id = ? AND soal_id = ?";
        let exists = sqlx::query(check_query)
            .bind(paket_soal_id)
            .bind(soal_id)
            .fetch_optional(&mut *tx)
            .await;
            
        match exists {
            Ok(Some(_)) => {
                // Mapping already exists, skip
                continue;
            }
            Ok(None) => {
                // Create new mapping
                let insert_query = "INSERT INTO dbquizapp.paket_soal_items (paket_soal_id, soal_id) VALUES (?, ?)";
                match sqlx::query(insert_query)
                    .bind(paket_soal_id)
                    .bind(soal_id)
                    .execute(&mut *tx)
                    .await
                {
                    Ok(_) => mapped_count += 1,
                    Err(err) => {
                        eprintln!("Error inserting mapping: {}", err);
                        let _ = tx.rollback().await;
                        return Ok(HttpResponse::InternalServerError().json("Failed to create mapping"));
                    }
                }
            }
            Err(err) => {
                eprintln!("Error checking existing mapping: {}", err);
                let _ = tx.rollback().await;
                return Ok(HttpResponse::InternalServerError().json("Database error"));
            }
        }
    }
    
    // Commit transaction
    match tx.commit().await {
        Ok(_) => {
            Ok(HttpResponse::Ok().json(MappingResponse {
                success: true,
                message: format!("{} questions mapped successfully", mapped_count),
                mapped_count,
            }))
        }
        Err(err) => {
            eprintln!("Error committing transaction: {}", err);
            Ok(HttpResponse::InternalServerError().json("Failed to commit changes"))
        }
    }
}

/// Remove questions from package mapping
#[utoipa::path(
    delete,
    path = "/admin/paket-soal/{paket_soal_id}/unmap-questions",
    params(
        ("paket_soal_id" = i32, Path, description = "Package ID")
    ),
    request_body = MappingRequest,
    responses(
        (status = 200, description = "Questions unmapped successfully", body = MappingResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Package Questions Mapping",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn unmap_questions_from_package(
    data: web::Data<AppState<'_>>,
    paket_soal_id: web::Path<i32>,
    request: web::Json<MappingRequest>,
    _http_req: HttpRequest,
) -> Result<HttpResponse> {
    let paket_soal_id = paket_soal_id.into_inner();
    let soal_ids = &request.soal_ids;
    
    if soal_ids.is_empty() {
        return Ok(HttpResponse::BadRequest().json("No questions provided"));
    }
    
    // Create placeholders for the IN clause
    let placeholders: Vec<String> = soal_ids.iter().map(|_| "?".to_string()).collect();
    let placeholders_str = placeholders.join(", ");
    
    let query = format!(
        "DELETE FROM dbquizapp.paket_soal_items WHERE paket_soal_id = ? AND soal_id IN ({})",
        placeholders_str
    );
    
    let mut query_builder = sqlx::query(&query);
    query_builder = query_builder.bind(paket_soal_id);
    
    for soal_id in soal_ids {
        query_builder = query_builder.bind(soal_id);
    }
    
    match query_builder.execute(&*data.context.soal.pool).await {
        Ok(result) => {
            let deleted_count = result.rows_affected() as i32;
            Ok(HttpResponse::Ok().json(MappingResponse {
                success: true,
                message: format!("{} questions unmapped successfully", deleted_count),
                mapped_count: deleted_count,
            }))
        }
        Err(err) => {
            eprintln!("Error unmapping questions: {}", err);
            Ok(HttpResponse::InternalServerError().json("Failed to unmap questions"))
        }
    }
}

/// Remove specific mapping by ID
#[utoipa::path(
    delete,
    path = "/admin/paket-soal-items/{id}",
    params(
        ("id" = i32, Path, description = "Mapping ID")
    ),
    responses(
        (status = 200, description = "Mapping removed successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Mapping not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Package Questions Mapping",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_mapping(
    data: web::Data<AppState<'_>>,
    mapping_id: web::Path<i32>,
    _http_req: HttpRequest,
) -> Result<HttpResponse> {
    let mapping_id = mapping_id.into_inner();
    
    let query = "DELETE FROM dbquizapp.paket_soal_items WHERE id = ?";
    
    match sqlx::query(query)
        .bind(mapping_id)
        .execute(&*data.context.soal.pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() == 0 {
                Ok(HttpResponse::NotFound().json("Mapping not found"))
            } else {
                Ok(HttpResponse::Ok().json("Mapping removed successfully"))
            }
        }
        Err(err) => {
            eprintln!("Error deleting mapping: {}", err);
            Ok(HttpResponse::InternalServerError().json("Failed to delete mapping"))
        }
    }
}