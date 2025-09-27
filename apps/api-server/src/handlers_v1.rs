use crate::domain::Domain;
use crate::models::{ErrorResponse, LoginRequest, RegisterRequest, UserResponse};
use crate::telemetry::Metrics;
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{HttpResponse, get, post, web};
use chrono::Utc;

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = String),
    )
)]
#[get("/health")]
pub async fn health(metrics_data: web::Data<Metrics>) -> HttpResponse {
    metrics_data.update_system_metrics();

    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "timestamp": Utc::now(),
        "uptime_seconds": metrics_data.uptime_seconds.get(),
        "active_connections": metrics_data.active_connections.get(),
        "active_sessions": metrics_data.active_sessions.get()
    }))
}

#[utoipa::path(
    get,
    path = "/metrics",
    tag = "health",
    responses(
        (status = 200, description = "Prometheus metrics", body = String),
    )
)]
#[get("/metrics")]
pub async fn metrics_endpoint(metrics: web::Data<Metrics>) -> HttpResponse {
    match metrics.export() {
        Ok(metrics_text) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4")
            .body(metrics_text),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: "metrics_error".to_string(),
            message: format!("Failed to export metrics: {e}"),
        }),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "auth",
    params(RegisterRequest),
    responses(
        (status = 201, description = "User registered successfully", body = UserResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
    )
)]
#[post("/auth/register")]
pub async fn register(
    query: web::Query<RegisterRequest>,
    domain: web::Data<Domain>,
    metrics: web::Data<Metrics>,
) -> HttpResponse {
    if let Err(err) = domain
        .register(
            &query.token,
            query.expires_at,
            &query.solana_wallet_public_key,
            &query.signature,
        )
        .await
    {
        metrics.record_user_registration(false);
        tracing::error!("{err}");
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "registration_failed".to_string(),
            message: "Failed to register user.".to_string(),
        });
    }
    metrics.record_user_registration(true);

    HttpResponse::Created().json(UserResponse {
        solana_wallet_public_key: query.solana_wallet_public_key.to_string(),
    })
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    params(LoginRequest),
    responses(
        (status = 200, description = "Login successful", body = UserResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
    )
)]
#[post("/auth/login")]
pub async fn login(
    query: web::Query<LoginRequest>,
    domain: web::Data<Domain>,
    metrics: web::Data<Metrics>,
) -> HttpResponse {
    match domain
        .login(
            &query.solana_wallet_public_key,
            &query.token,
            query.expires_at,
            &query.signature,
        )
        .await
    {
        Ok(token) => {
            metrics.record_auth_attempt("login", true);
            metrics.record_user_login(true);
            metrics.active_sessions.inc();
            let cookie = Cookie::build("auth_token", token.clone())
                .path("/")
                .http_only(true)
                .same_site(SameSite::Strict)
                .secure(true)
                .finish();
            HttpResponse::Ok().cookie(cookie).json(UserResponse {
                solana_wallet_public_key: query.solana_wallet_public_key.to_string(),
            })
        }
        Err(err) => {
            metrics.record_auth_attempt("login", false);
            metrics.record_user_login(false);
            metrics
                .api_errors_by_type
                .with_label_values(&["token_generation_failed", "/api/v1/auth/login"])
                .inc();
            tracing::error!("{err}");
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "login_failed".to_string(),
                message: "Failed to generate authentication token".to_string(),
            })
        }
    }
}
