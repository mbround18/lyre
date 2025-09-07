use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{Ready, ready},
    rc::Rc,
};

use crate::auth::{AuthenticatedUser, get_user_guilds, validate_discord_token};

pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            // Skip authentication for certain paths
            let path = req.path();
            if should_skip_auth(path) {
                return service.call(req).await;
            }

            // Extract token from Authorization header
            match extract_token_from_request(&req) {
                Some(token) => {
                    // Validate token and get user data
                    match validate_token_and_get_user(&token).await {
                        Ok(user) => {
                            // Store authenticated user in request extensions
                            req.extensions_mut().insert(user);
                            service.call(req).await
                        }
                        Err(e) => {
                            tracing::warn!("Token validation failed: {}", e);
                            Err(actix_web::error::ErrorUnauthorized(
                                "Invalid or expired token",
                            ))
                        }
                    }
                }
                None => {
                    tracing::warn!("No authorization token found in request to {}", path);
                    Err(actix_web::error::ErrorUnauthorized(
                        "Missing authorization token",
                    ))
                }
            }
        })
    }
}

fn should_skip_auth(path: &str) -> bool {
    // Skip authentication for these paths
    path.starts_with("/static")
        || path.starts_with("/auth")
        || path.starts_with("/api/health")
        || path.starts_with("/api/livez")
        || path.starts_with("/api/readyz")
        || path.starts_with("/api/dev/test-token")
        || path.starts_with("/api/auth/validate")
        || path == "/"
        || path == "/favicon.ico"
}

fn extract_token_from_request(req: &ServiceRequest) -> Option<String> {
    req.headers()
        .get("Authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

async fn validate_token_and_get_user(
    token: &str,
) -> Result<AuthenticatedUser, Box<dyn std::error::Error>> {
    // Validate real Discord token
    let user = validate_discord_token(token).await.map_err(|e| {
        tracing::warn!("Discord token validation error: {}", e);
        e
    })?;
    let guilds = get_user_guilds(token).await?;

    Ok(AuthenticatedUser { user, guilds })
}
