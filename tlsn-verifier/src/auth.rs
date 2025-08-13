use actix_web::body::BoxBody;
use actix_web::{dev::ServiceRequest, Error, HttpResponse};
use actix_web::dev::{Service, Transform};
use futures_util::future::{ok, Ready, LocalBoxFuture};
use std::rc::Rc;
use crate::config;

/// Middleware struct for API key-based authorization
pub struct ApiKeyAuth;

/// Implements the `Transform` trait to wrap services with `ApiKeyAuthMiddleware`
impl<S, B> Transform<S, ServiceRequest> for ApiKeyAuth
where
    S: Service<ServiceRequest, Response = actix_web::dev::ServiceResponse<B>, Error = Error> + 'static,
    B: actix_web::body::MessageBody + 'static, // Ensure message body type is compatible
{
    // Output type of the wrapped service
    type Response = actix_web::dev::ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = ApiKeyAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    /// Called once during application startup to create the middleware
    fn new_transform(&self, service: S) -> Self::Future {
        ok(ApiKeyAuthMiddleware {
            service: Rc::new(service), // Store wrapped service in a reference-counted pointer
        })
    }
}

/// Middleware logic for API key verification
pub struct ApiKeyAuthMiddleware<S> {
    service: Rc<S>, // Wrapped service
}

/// Implements `Service` trait for the middleware to intercept requests
impl<S, B> Service<ServiceRequest> for ApiKeyAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = actix_web::dev::ServiceResponse<B>, Error = Error> + 'static,
    B: actix_web::body::MessageBody + 'static,
{
    // Define the response and error types
    type Response = actix_web::dev::ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    /// Polls if the service is ready to process requests
    fn poll_ready(&self, ctx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    /// Handles the incoming request with API key authentication
    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Retrieve expected API key from config
        let api_key = config::get_api_key();

        // Extract "x-api-key" header and compare it to expected key
        let authorized = req
            .headers()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map_or(false, |key| key == api_key);

        // Clone the service so it can be used inside async block
        let srv = self.service.clone();

        // Return a boxed future handling authorization
        Box::pin(async move {
            if authorized {
                // If key matches, forward request to inner service
                let res = srv.call(req).await?;
                Ok(res.map_into_boxed_body())
            } else {
                // If unauthorized, return 401 Unauthorized response
                let res = req.into_response(HttpResponse::Unauthorized().finish());
                Ok(res.map_into_boxed_body())
            }
        })
    }
}
