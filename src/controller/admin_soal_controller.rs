use crate::controller::log_request;
use crate::middleware::admin_middleware::AdminMiddleware;
use crate::model::soal::{Soal, AdminSoal, UpdateSoalRequest, QuestionSearchRequest, PaginatedQuestionsResponse, BulkImportRequest, BulkImportResponse, CreateSoalRequest, CsvImportRequest, CsvImportResponse};
use crate::service::csv_import_service::{CsvImportService, CsvImportConfig};
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest, get, post, put, delete};
use actix_multipart::Multipart;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::io::Write;
use tempfile::NamedTempFile;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/soal")
            .wrap(AdminMiddleware::new())
            .service(search_questions)
            .service(list_questions)
            .service(get_dropdowns)
            .service(create_question)
            .service(update_question)
            .service(delete_question)
            .service(bulk_import_questions)
            .service(upload_csv_preview)
            .service(upload_csv_import)
            .service(download_csv_template)
            .service(get_question_by_id)
    );
}

/// Search questions with pagination and filtering
#[utoipa::path(
    get,
    path = "/admin/soal/search",
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("limit" = Option<u32>, Query, description = "Items per page (default: 20)"),
        ("search" = Option<String>, Query, description = "Search in question text"),
        ("modul" = Option<String>, Query, description = "Filter by module"),
        ("pelajaran" = Option<String>, Query, description = "Filter by subject"),
        ("tag" = Option<String>, Query, description = "Filter by tag")
    ),
    responses(
        (status = 200, description = "Questions retrieved successfully", body = PaginatedQuestionsResponse),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/search")]
async fn search_questions(
    query: web::Query<QuestionSearchRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/soal/search", &data.connections);

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    // Build dynamic query
    let mut where_conditions = vec!["1=1"];
    let mut bind_values: Vec<String> = vec![];

    if let Some(ref search) = query.search {
        if !search.is_empty() {
            where_conditions.push("s.soal LIKE ?");
            bind_values.push(format!("%{}%", search));
        }
    }

    if let Some(ref modul) = query.modul {
        where_conditions.push("s.modul = ?");
        bind_values.push(modul.clone());
    }

    if let Some(ref pelajaran) = query.pelajaran {
        where_conditions.push("s.pelajaran = ?");
        bind_values.push(pelajaran.clone());
    }

    if let Some(ref tag) = query.tag {
        where_conditions.push("s.tag = ?");
        bind_values.push(tag.clone());
    }

    let where_clause = where_conditions.join(" AND ");

    // Count total questions
    let count_query = format!(
        "SELECT COUNT(*) FROM dbquizapp.soal s WHERE {}",
        where_clause
    );

    let mut count_query_builder = sqlx::query_scalar::<_, i64>(&count_query);
    for value in &bind_values {
        count_query_builder = count_query_builder.bind(value);
    }

    let total: i64 = match count_query_builder
        .fetch_one(&*data.context.soal.pool)
        .await 
    {
        Ok(count) => count,
        Err(e) => {
            println!("Error counting questions: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to count questions".to_string(),
            });
        }
    };

    // Get questions
    let questions_query = format!(
        r#"
        SELECT 
            s.*,
            COALESCE(s.created_at, NOW()) as created_at,
            COALESCE(s.updated_at, NOW()) as updated_at,
            0 as usage_count
        FROM dbquizapp.soal s
        WHERE {}
        ORDER BY s.id DESC
        LIMIT ? OFFSET ?
        "#,
        where_clause
    );

    let mut questions_query_builder = sqlx::query_as::<_, AdminSoal>(&questions_query);
    for value in &bind_values {
        questions_query_builder = questions_query_builder.bind(value);
    }

    let questions = match questions_query_builder
        .bind(limit)
        .bind(offset)
        .fetch_all(&*data.context.soal.pool)
        .await 
    {
        Ok(questions) => questions,
        Err(e) => {
            println!("Error fetching questions: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch questions".to_string(),
            });
        }
    };

    let total_pages = (total as f64 / limit as f64).ceil() as u32;

    HttpResponse::Ok().json(PaginatedQuestionsResponse {
        questions,
        total,
        page,
        limit,
        total_pages,
    })
}

/// Get question by ID
#[utoipa::path(
    get,
    path = "/admin/soal/{id}",
    params(
        ("id" = i32, Path, description = "Question ID")
    ),
    responses(
        (status = 200, description = "Question found", body = AdminSoal),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Question not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/{id}")]
async fn get_question_by_id(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/soal/{id}", &data.connections);

    let question_id = path.into_inner();
    
    let query = r#"
        SELECT 
            s.*,
            COALESCE(s.created_at, NOW()) as created_at,
            COALESCE(s.updated_at, NOW()) as updated_at,
            0 as usage_count
        FROM dbquizapp.soal s
        WHERE s.id = ?
    "#;

    match sqlx::query_as::<_, AdminSoal>(query)
        .bind(question_id)
        .fetch_one(&*data.context.soal.pool)
        .await 
    {
        Ok(question) => HttpResponse::Ok().json(question),
        Err(e) => {
            println!("Error fetching question: {:?}", e);
            HttpResponse::NotFound().json(ErrorResponse {
                error: "Question not found".to_string(),
            })
        }
    }
}

/// Create new question
#[utoipa::path(
    post,
    path = "/admin/soal",
    request_body = CreateSoalRequest,
    responses(
        (status = 201, description = "Question created successfully", body = Soal),
        (status = 400, description = "Invalid question data"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("")]
async fn create_question(
    question_req: web::Json<CreateSoalRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/soal", &data.connections);

    // Validate question data
    if question_req.soal.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Question text cannot be empty".to_string(),
        });
    }

    match data.context.soal.create_soal(&*question_req).await {
        Ok(question) => HttpResponse::Created().json(question),
        Err(e) => {
            println!("Error creating question: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to create question".to_string(),
            })
        }
    }
}

/// Update question
#[utoipa::path(
    put,
    path = "/admin/soal/{id}",
    params(
        ("id" = i32, Path, description = "Question ID")
    ),
    request_body = UpdateSoalRequest,
    responses(
        (status = 200, description = "Question updated successfully", body = Soal),
        (status = 400, description = "Invalid question data"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Question not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[put("/{id}")]
async fn update_question(
    path: web::Path<i32>,
    question_req: web::Json<UpdateSoalRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("PUT /admin/soal/{id}", &data.connections);

    let question_id = path.into_inner();

    // Validate question data
    if question_req.soal.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Question text cannot be empty".to_string(),
        });
    }

    let result = sqlx::query(
        r#"
        UPDATE dbquizapp.soal 
        SET soal = ?, opt1 = ?, opt2 = ?, opt3 = ?, opt4 = ?, opt5 = ?,
            correct_answer = ?, solution = ?, sumberfile = ?, modul = ?, 
            pelajaran = ?, tag = ?, updated_at = NOW()
        WHERE id = ?
        "#
    )
    .bind(&question_req.soal)
    .bind(&question_req.opt1)
    .bind(&question_req.opt2)
    .bind(&question_req.opt3)
    .bind(&question_req.opt4)
    .bind(&question_req.opt5)
    .bind(&question_req.correct_answer)
    .bind(&question_req.solution)
    .bind(&question_req.sumberfile)
    .bind(&question_req.modul)
    .bind(&question_req.pelajaran)
    .bind(&question_req.tag)
    .bind(question_id)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                // Fetch the updated question
                match data.context.soal.get_soal_by_id(&question_id.to_string()).await {
                    Ok(question) => HttpResponse::Ok().json(question),
                    Err(e) => {
                        println!("Error fetching updated question: {:?}", e);
                        HttpResponse::InternalServerError().json(ErrorResponse {
                            error: "Question updated but failed to fetch details".to_string(),
                        })
                    }
                }
            } else {
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "Question not found".to_string(),
                })
            }
        }
        Err(e) => {
            println!("Error updating question: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to update question".to_string(),
            })
        }
    }
}

/// Delete question
#[utoipa::path(
    delete,
    path = "/admin/soal/{id}",
    params(
        ("id" = i32, Path, description = "Question ID")
    ),
    responses(
        (status = 204, description = "Question deleted successfully"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Question not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[delete("/{id}")]
async fn delete_question(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("DELETE /admin/soal/{id}", &data.connections);

    let question_id = path.into_inner();

    let result = sqlx::query(
        "DELETE FROM dbquizapp.soal WHERE id = ?"
    )
    .bind(question_id)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                HttpResponse::NoContent().finish()
            } else {
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "Question not found".to_string(),
                })
            }
        }
        Err(e) => {
            println!("Error deleting question: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to delete question".to_string(),
            })
        }
    }
}

/// Bulk import questions
#[utoipa::path(
    post,
    path = "/admin/soal/bulk-import",
    request_body = BulkImportRequest,
    responses(
        (status = 200, description = "Bulk import completed", body = BulkImportResponse),
        (status = 400, description = "Invalid bulk import data"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("/bulk-import")]
async fn bulk_import_questions(
    import_req: web::Json<BulkImportRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/soal/bulk-import", &data.connections);

    if import_req.questions.is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "No questions provided for import".to_string(),
        });
    }

    let mut success_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    for (index, question) in import_req.questions.iter().enumerate() {
        // Validate question data
        if question.soal.trim().is_empty() {
            failed_count += 1;
            errors.push(format!("Question {}: Question text cannot be empty", index + 1));
            continue;
        }

        let result = data.context.soal.create_soal(question).await;

        match result {
            Ok(_) => success_count += 1,
            Err(e) => {
                failed_count += 1;
                errors.push(format!("Question {}: {}", index + 1, e.to_string()));
            }
        }
    }

    HttpResponse::Ok().json(BulkImportResponse {
        success_count,
        failed_count,
        errors,
    })
}

/// List all questions (comprehensive endpoint similar to get_list_paket_soal_lengkap)
#[utoipa::path(
    get,
    path = "/admin/soal/questions",
    responses(
        (status = 200, description = "Questions retrieved successfully", body = [AdminSoal]),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/questions")]
async fn list_questions(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/soal/questions", &data.connections);
    
    let questions = data.context.soal.get_all_soal().await;

    match questions {
        Err(e) => {
            eprintln!("Error fetching questions: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch questions".to_string(),
            })
        },
        Ok(questions) => HttpResponse::Ok().json(questions),
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DropdownData {
    pub modules: Vec<String>,
    pub subjects: Vec<String>,
    pub tags: Vec<String>,
}

/// Get dropdown data for questions form
#[utoipa::path(
    get,
    path = "/admin/soal/dropdowns",
    responses(
        (status = 200, description = "Dropdown data retrieved successfully", body = DropdownData),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/dropdowns")]
async fn get_dropdowns(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/soal/dropdowns", &data.connections);

    // Get distinct modules
    let modules_query = r#"
        SELECT DISTINCT modul 
        FROM dbquizapp.soal 
        WHERE modul IS NOT NULL AND modul != '' 
        ORDER BY modul ASC
    "#;

    let modules = match sqlx::query_scalar::<_, String>(modules_query)
        .fetch_all(&*data.context.soal.pool)
        .await 
    {
        Ok(modules) => modules,
        Err(e) => {
            println!("Error fetching modules: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch modules".to_string(),
            });
        }
    };

    // Get distinct subjects
    let subjects_query = r#"
        SELECT DISTINCT pelajaran 
        FROM dbquizapp.soal 
        WHERE pelajaran IS NOT NULL AND pelajaran != '' 
        ORDER BY pelajaran ASC
    "#;

    let subjects = match sqlx::query_scalar::<_, String>(subjects_query)
        .fetch_all(&*data.context.soal.pool)
        .await 
    {
        Ok(subjects) => subjects,
        Err(e) => {
            println!("Error fetching subjects: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch subjects".to_string(),
            });
        }
    };

    // Get distinct tags
    let tags_query = r#"
        SELECT DISTINCT tag 
        FROM dbquizapp.soal 
        WHERE tag IS NOT NULL AND tag != '' 
        ORDER BY tag ASC
    "#;

    let tags = match sqlx::query_scalar::<_, String>(tags_query)
        .fetch_all(&*data.context.soal.pool)
        .await 
    {
        Ok(tags) => tags,
        Err(e) => {
            println!("Error fetching tags: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch tags".to_string(),
            });
        }
    };

    HttpResponse::Ok().json(DropdownData {
        modules,
        subjects,
        tags,
    })
}

/// Upload CSV file for preview and validation
#[utoipa::path(
    post,
    path = "/admin/soal/upload-csv-preview",
    request_body(
        content = String,
        description = "CSV file content",
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "CSV preview generated successfully"),
        (status = 400, description = "Invalid CSV format or content"),
        (status = 403, description = "Admin access required"),
        (status = 413, description = "File too large"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("/upload-csv-preview")]
async fn upload_csv_preview(
    mut payload: Multipart,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/soal/upload-csv-preview", &data.connections);

    let csv_service = CsvImportService::default();
    
    // Process multipart form data
    while let Some(mut field) = payload.try_next().await.unwrap_or(None) {
        let content_disposition = field.content_disposition();
        
        if let Some(name) = content_disposition.get_name() {
            if name == "file" {
                // Read file content
                let mut file_content = Vec::new();
                while let Some(chunk) = field.try_next().await.unwrap_or(None) {
                    file_content.extend_from_slice(&chunk);
                }

                // Validate file
                if let Err(e) = csv_service.validate_file(&file_content) {
                    return HttpResponse::BadRequest().json(ErrorResponse {
                        error: e,
                    });
                }

                // Convert to string
                let csv_content = match std::str::from_utf8(&file_content) {
                    Ok(content) => content,
                    Err(_) => {
                        return HttpResponse::BadRequest().json(ErrorResponse {
                            error: "File must be valid UTF-8 text".to_string(),
                        });
                    }
                };

                // Parse and generate preview
                match csv_service.parse_csv_preview(csv_content).await {
                    Ok(preview) => {
                        return HttpResponse::Ok().json(preview);
                    }
                    Err(e) => {
                        return HttpResponse::BadRequest().json(ErrorResponse {
                            error: e,
                        });
                    }
                }
            }
        }
    }

    HttpResponse::BadRequest().json(ErrorResponse {
        error: "No file provided in multipart form data".to_string(),
    })
}

/// Upload and import CSV file
#[utoipa::path(
    post,
    path = "/admin/soal/upload-csv-import",
    request_body(
        content = String,
        description = "CSV file content with import options",
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "CSV imported successfully"),
        (status = 400, description = "Invalid CSV format or import failed"),
        (status = 403, description = "Admin access required"),
        (status = 413, description = "File too large"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("/upload-csv-import")]
async fn upload_csv_import(
    mut payload: Multipart,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/soal/upload-csv-import", &data.connections);

    let csv_service = CsvImportService::default();
    let mut csv_content: Option<String> = None;
    let mut import_options = CsvImportRequest {
        skip_invalid_rows: true,
        max_errors: Some(100),
        validate_only: false,
    };
    
    // Process multipart form data
    while let Some(mut field) = payload.try_next().await.unwrap_or(None) {
        let content_disposition = field.content_disposition();
        
        if let Some(name) = content_disposition.get_name() {
            match name {
                "file" => {
                    // Read file content
                    let mut file_content = Vec::new();
                    while let Some(chunk) = field.try_next().await.unwrap_or(None) {
                        file_content.extend_from_slice(&chunk);
                    }

                    // Validate file
                    if let Err(e) = csv_service.validate_file(&file_content) {
                        return HttpResponse::BadRequest().json(ErrorResponse {
                            error: e,
                        });
                    }

                    // Convert to string
                    match std::str::from_utf8(&file_content) {
                        Ok(content) => csv_content = Some(content.to_string()),
                        Err(_) => {
                            return HttpResponse::BadRequest().json(ErrorResponse {
                                error: "File must be valid UTF-8 text".to_string(),
                            });
                        }
                    }
                }
                "options" => {
                    // Read import options JSON
                    let mut options_content = Vec::new();
                    while let Some(chunk) = field.try_next().await.unwrap_or(None) {
                        options_content.extend_from_slice(&chunk);
                    }
                    
                    if let Ok(options_str) = std::str::from_utf8(&options_content) {
                        if let Ok(parsed_options) = serde_json::from_str::<CsvImportRequest>(options_str) {
                            import_options = parsed_options;
                        }
                    }
                }
                _ => {} // Ignore other fields
            }
        }
    }

    let csv_content = match csv_content {
        Some(content) => content,
        None => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "No CSV file provided".to_string(),
            });
        }
    };

    // Parse CSV and get preview
    let preview = match csv_service.parse_csv_preview(&csv_content).await {
        Ok(preview) => preview,
        Err(e) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: e,
            });
        }
    };

    // If validate_only is true, return preview
    if import_options.validate_only {
        return HttpResponse::Ok().json(preview);
    }

    // Extract questions to import
    let questions_to_import = if import_options.skip_invalid_rows {
        csv_service.extract_questions_from_preview(&preview)
    } else {
        if !preview.invalid_questions.is_empty() {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("CSV contains {} invalid rows. Fix errors or set skip_invalid_rows=true", preview.invalid_questions.len()),
            });
        }
        csv_service.extract_questions_from_preview(&preview)
    };

    if questions_to_import.is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "No valid questions found in CSV file".to_string(),
        });
    }

    // Import questions using existing bulk import logic
    let mut success_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    for (index, question) in questions_to_import.iter().enumerate() {
        // Validate question data
        if question.soal.trim().is_empty() {
            failed_count += 1;
            errors.push(format!("Question {}: Question text cannot be empty", index + 1));
            continue;
        }

        let result = data.context.soal.create_soal(question).await;
        match result {
            Ok(_) => success_count += 1,
            Err(e) => {
                failed_count += 1;
                errors.push(format!("Question {}: {}", index + 1, e));
                
                // Check max errors limit
                if let Some(max_errors) = import_options.max_errors {
                    if errors.len() >= max_errors {
                        break;
                    }
                }
            }
        }
    }

    let response = CsvImportResponse {
        success_count,
        failed_count,
        skipped_count: preview.invalid_questions.len() as i32,
        total_processed: (success_count + failed_count) as i32,
        errors: errors.into_iter().map(|msg| crate::model::soal::CsvImportError {
            row_number: 0, // Would need more detailed tracking
            field: "general".to_string(),
            error_type: "import_error".to_string(),
            message: msg,
            suggested_fix: None,
            raw_value: None,
        }).collect(),
        warnings: vec![], // Could add warnings from preview
        import_id: None,   // Could generate unique import ID
        estimated_time_seconds: 0, // Already completed
    };

    HttpResponse::Ok().json(response)
}

/// Download CSV template for bulk import
#[utoipa::path(
    get,
    path = "/admin/soal/download-csv-template",
    responses(
        (status = 200, description = "CSV template downloaded successfully", content_type = "text/csv"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/download-csv-template")]
async fn download_csv_template(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/soal/download-csv-template", &data.connections);

    let template_content = r#"question_text,option_1,option_2,option_3,option_4,option_5,correct_answer,solution,module,subject,tag,source_file
"What is 2+2?","2","3","4","5","","3","2+2 equals 4 because it's basic arithmetic","Basic Math","Mathematics","arithmetic","math-basics.pdf"
"Which planet is closest to the Sun?","Venus","Mercury","Earth","Mars","","2","Mercury is the closest planet to the Sun","Solar System","Science","astronomy,planets","science-101.pdf"
"True or False: Paris is the capital of France","True","False","","","","1","Paris is indeed the capital and largest city of France","Geography","Geography","capitals,europe","geography.pdf"
"Select the largest ocean","Atlantic","Pacific","Indian","Arctic","","2","The Pacific Ocean is the largest ocean on Earth","Oceans","Geography","ocean,geography","earth-science.pdf"
"What is the result of 5 × 6?","25","30","35","40","","2","5 × 6 = 30. Multiplication of 5 and 6.","Multiplication","Mathematics","multiplication,basic","math-fundamentals.pdf""#;

    HttpResponse::Ok()
        .content_type("text/csv")
        .insert_header(("Content-Disposition", "attachment; filename=questions_template.csv"))
        .body(template_content)
}