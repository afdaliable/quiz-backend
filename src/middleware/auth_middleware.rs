use actix_web::{dev::{Service, ServiceRequest, ServiceResponse, Transform}, Error, HttpMessage};
use actix_web::http::header;
use futures::future::{LocalBoxFuture, Ready};
use std::task::{Context, Poll};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    aud: String,
    exp: usize,
    email: String,
    role: String,
    session_id: String,
}

pub struct AuthMiddleware {
    jwt_secret: String,
}

impl AuthMiddleware {
    pub fn new(jwt_secret: String) -> Self {
        AuthMiddleware { jwt_secret }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        futures::future::ready(Ok(AuthMiddlewareService {
            service,
            jwt_secret: self.jwt_secret.clone(),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: S,
    jwt_secret: String,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Skip auth for signup and login endpoints
        if req.path() == "/signup" || req.path() == "/login" {
            return Box::pin(self.service.call(req));
        }

        let auth_header = req.headers().get(header::AUTHORIZATION);
        let jwt_secret = self.jwt_secret.clone();
        
        match auth_header {
            Some(auth_str) => {
                if let Ok(auth_str) = auth_str.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = &auth_str[7..];
                        
                        let mut validation = Validation::new(Algorithm::HS256);
                        validation.validate_exp = true;
                        validation.set_audience(&["authenticated"]);
                        
                        match decode::<Claims>(
                            token,
                            &DecodingKey::from_secret(jwt_secret.as_bytes()),
                            &validation
                        ) {
                            Ok(token_data) => {
                                req.extensions_mut().insert(token_data.claims);
                                return Box::pin(self.service.call(req));
                            },
                            Err(e) => {
                                eprintln!("Token validation error: {:?}", e);
                                return Box::pin(async move {
                                    Err(actix_web::error::ErrorUnauthorized("Invalid token"))
                                });
                            }
                        }
                    }
                }
                Box::pin(async move {
                    Err(actix_web::error::ErrorUnauthorized("Invalid authorization header"))
                })
            }
            None => Box::pin(async move {
                Err(actix_web::error::ErrorUnauthorized("Missing authorization header"))
            })
        }
    }
}