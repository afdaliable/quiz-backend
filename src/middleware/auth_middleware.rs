use actix_web::{dev::{Service, ServiceRequest, ServiceResponse, Transform}, Error};
use actix_web::http::header;
use futures::future::{LocalBoxFuture, Ready};
use std::task::{Context, Poll};




pub struct AuthMiddleware {
    api_key: String,
}

impl AuthMiddleware {
    pub fn new(api_key: String) -> Self {
        AuthMiddleware { api_key }
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
            api_key: self.api_key.clone(),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: S,
    api_key: String,
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
        let api_key = self.api_key.clone();
        let is_authorized = req
            .headers()
            .get(header::AUTHORIZATION)
            .map(|auth_header| auth_header.as_bytes() == api_key.as_bytes())
            .unwrap_or(false);

        if is_authorized {
            Box::pin(self.service.call(req))
        } else {
            Box::pin(async move {
                Err(actix_web::error::ErrorUnauthorized("Unauthorized"))
            })
        }
    }
}