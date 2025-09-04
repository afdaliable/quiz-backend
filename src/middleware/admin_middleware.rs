use actix_web::{dev::{Service, ServiceRequest, ServiceResponse, Transform}, Error, HttpMessage, http::StatusCode};
use actix_web::http::header;
use futures::future::{LocalBoxFuture, Ready};
use std::task::{Context, Poll};
use std::rc::Rc;
use std::cell::RefCell;
use serde_json::json;
// use crate::utils::auth::extract_user_id;
use crate::AppState;

pub struct AdminMiddleware;

impl AdminMiddleware {
    pub fn new() -> Self {
        AdminMiddleware
    }
}

impl<S, B> Transform<S, ServiceRequest> for AdminMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AdminMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        futures::future::ready(Ok(AdminMiddlewareService {
            service: Rc::new(RefCell::new(service)),
        }))
    }
}

pub struct AdminMiddlewareService<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service<ServiceRequest> for AdminMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.borrow_mut().poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        println!("Admin middleware: Checking admin access for path: {}", req.path());
        
        // Extract user ID from the authenticated request
        let user_id_opt = req.headers()
            .get("user_id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let service = self.service.clone();

        Box::pin(async move {
            if let Some(user_id) = user_id_opt {
                // Get app state to check user role in database
                if let Some(app_data) = req.app_data::<actix_web::web::Data<AppState>>() {
                    let app_data_clone = app_data.clone();
                    
                    // Check if user has admin role
                    println!("Admin middleware: Checking role for user_id: {}", user_id);
                    match check_admin_role(&app_data_clone, &user_id).await {
                        Ok(true) => {
                            // User is admin, proceed with request
                            println!("Admin middleware: User {} is admin, proceeding with request", user_id);
                            service.borrow_mut().call(req).await
                        },
                        Ok(false) => {
                            // User is not admin
                            println!("Admin middleware: User {} is NOT admin, denying access", user_id);
                            let error_response = json!({
                                "error": "insufficient_permissions",
                                "message": "You don't have admin permissions to access this resource.",
                                "status_code": 403
                            });
                            
                            Err(actix_web::error::InternalError::new(
                                error_response,
                                StatusCode::FORBIDDEN,
                            ).into())
                        },
                        Err(e) => {
                            println!("Error checking admin role: {:?}", e);
                            let error_response = json!({
                                "error": "database_error",
                                "message": "Error verifying admin permissions.",
                                "status_code": 500
                            });
                            
                            Err(actix_web::error::InternalError::new(
                                error_response,
                                StatusCode::INTERNAL_SERVER_ERROR,
                            ).into())
                        }
                    }
                } else {
                    let error_response = json!({
                        "error": "app_state_error",
                        "message": "Unable to verify admin permissions.",
                        "status_code": 500
                    });
                    
                    Err(actix_web::error::InternalError::new(
                        error_response,
                        StatusCode::INTERNAL_SERVER_ERROR,
                    ).into())
                }
            } else {
                // No user ID found (should not happen if auth middleware ran first)
                let error_response = json!({
                    "error": "unauthenticated",
                    "message": "Authentication required for admin access.",
                    "status_code": 401
                });
                
                Err(actix_web::error::InternalError::new(
                    error_response,
                    StatusCode::UNAUTHORIZED,
                ).into())
            }
        })
    }
}

// Helper function to check if user has admin role
async fn check_admin_role(app_state: &actix_web::web::Data<AppState<'_>>, user_id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query_scalar::<_, Option<String>>(
        "SELECT role FROM dbquizapp.users WHERE id = ? AND deleted_at IS NULL"
    )
    .bind(user_id)
    .fetch_optional(&*app_state.context.users.pool)
    .await?;
    
    match result {
        Some(Some(role)) => Ok(role == "admin" || role == "superadmin"),
        _ => Ok(false), // User not found or has no role
    }
}