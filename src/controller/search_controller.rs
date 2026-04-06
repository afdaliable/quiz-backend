use crate::controller::log_request;
use crate::model::search::{SearchFiltersResponse, SoalSearchQuery, SoalSearchResponse};
use crate::AppState;
use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/soal")
            .route("/search", web::get().to(search_soal))
            .route("/search/filters", web::get().to(search_filters)),
    );
}

// GET /soal/search?q=&modul=&pelajaran=&page=&limit=
async fn search_soal(
    query: web::Query<SoalSearchQuery>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET /soal/search", &data.connections);

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(50);
    let offset = (page - 1) * limit;

    // Build WHERE conditions
    let mut where_conditions: Vec<&str> = vec!["1=1"];
    let mut bind_values: Vec<String> = vec![];

    if let Some(ref q) = query.q {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            where_conditions.push("(s.soal LIKE ? OR s.solution LIKE ?)");
            bind_values.push(format!("%{}%", trimmed));
            bind_values.push(format!("%{}%", trimmed));
        }
    }

    if let Some(ref modul) = query.modul {
        let trimmed = modul.trim();
        if !trimmed.is_empty() {
            where_conditions.push("s.modul = ?");
            bind_values.push(trimmed.to_string());
        }
    }

    if let Some(ref pelajaran) = query.pelajaran {
        let trimmed = pelajaran.trim();
        if !trimmed.is_empty() {
            where_conditions.push("s.pelajaran = ?");
            bind_values.push(trimmed.to_string());
        }
    }

    let where_clause = where_conditions.join(" AND ");

    // COUNT query (count distinct soal IDs)
    let count_sql = format!(
        "SELECT COUNT(DISTINCT s.id) FROM dbquizapp.soal s WHERE {}",
        where_clause
    );

    let mut count_builder = sqlx::query_scalar::<_, i64>(&count_sql);
    for val in &bind_values {
        count_builder = count_builder.bind(val);
    }

    let total = match count_builder.fetch_one(&*data.context.soal.pool).await {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error counting soal search results: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to count results".to_string(),
            });
        }
    };

    // Main query — GROUP BY to deduplicate soal that appear in multiple packages
    let search_sql = format!(
        r#"
        SELECT
            s.id,
            LEFT(s.soal, 200) AS soal,
            s.question_type,
            s.modul,
            s.pelajaran,
            s.tag,
            MAX(ps.nama_paket_soal) AS nama_paket_soal,
            MAX(ks.nama_kategori)   AS kategori_soal
        FROM dbquizapp.soal s
        LEFT JOIN dbquizapp.paket_soal_items psi ON s.id = psi.soal_id
        LEFT JOIN dbquizapp.paket_soal ps ON psi.paket_soal_id = ps.id
        LEFT JOIN dbquizapp.kategori_soal ks ON ps.kategori_id = ks.id
        WHERE {}
        GROUP BY s.id, s.soal, s.question_type, s.modul, s.pelajaran, s.tag
        ORDER BY s.id DESC
        LIMIT ? OFFSET ?
        "#,
        where_clause
    );

    let mut search_builder =
        sqlx::query_as::<_, crate::model::search::SoalSearchResult>(&search_sql);
    for val in &bind_values {
        search_builder = search_builder.bind(val);
    }
    search_builder = search_builder.bind(limit).bind(offset);

    let results = match search_builder.fetch_all(&*data.context.soal.pool).await {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error searching soal: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to search soal".to_string(),
            });
        }
    };

    HttpResponse::Ok().json(SoalSearchResponse {
        results,
        total,
        page,
        limit,
    })
}

// GET /soal/search/filters
async fn search_filters(data: web::Data<AppState<'_>>) -> impl Responder {
    log_request("GET /soal/search/filters", &data.connections);

    let modul_sql = r#"
        SELECT DISTINCT modul
        FROM dbquizapp.soal
        WHERE modul IS NOT NULL AND modul != ''
        ORDER BY modul ASC
    "#;

    let modul: Vec<String> = match sqlx::query_scalar::<_, String>(modul_sql)
        .fetch_all(&*data.context.soal.pool)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching modul filters: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch filters".to_string(),
            });
        }
    };

    let pelajaran_sql = r#"
        SELECT DISTINCT pelajaran
        FROM dbquizapp.soal
        WHERE pelajaran IS NOT NULL AND pelajaran != ''
        ORDER BY pelajaran ASC
    "#;

    let pelajaran: Vec<String> = match sqlx::query_scalar::<_, String>(pelajaran_sql)
        .fetch_all(&*data.context.soal.pool)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching pelajaran filters: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch filters".to_string(),
            });
        }
    };

    HttpResponse::Ok().json(SearchFiltersResponse { modul, pelajaran })
}
