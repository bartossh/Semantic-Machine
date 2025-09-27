use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use anyhow::Context;
use anyhow::anyhow;
use auth::Authenticator;
use config::Config;
use database::PostgresStorageGateway;
use domain::Domain;
use dotenv::dotenv;
use sqlx::migrate::Migrator;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::time::Duration;
use telemetry::Metrics;
use tokio::time::interval;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod auth;
mod config;
mod constants;
mod database;
mod domain;
mod handlers_v1;
mod middleware_v1;
mod models;
mod telemetry;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers_v1::register,
        handlers_v1::login,
        handlers_v1::health,
        handlers_v1::metrics_endpoint
    ),
    components(
        schemas(
            models::UserResponse,
            models::Claims,
            models::ErrorResponse
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "health", description = "Health check endpoints")
    ),
    info(
        title = "Semantic Machine API",
        version = "1.0.0",
        description = "REST API for Semantic-Machine services",
        contact(
            name = "Bartosz Lenart",
            email = "bartossh@pm.me"
        ),
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
        (url = "https://semantic-machine-dev.up.railway.app", description = "Development server")
    )
)]
struct ApiDoc;

#[inline(always)]
#[allow(clippy::io_other_error)]
fn to_io_error(e: anyhow::Error) -> Error {
    Error::new(ErrorKind::Other, format!("{e}"))
}

/// Start a background task to periodically update system metrics
#[inline(always)]
async fn start_metrics_updater(metrics: Arc<Metrics>) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(10));
        loop {
            ticker.tick().await;
            metrics.update_system_metrics();
        }
    });
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let config = Config::from_env().expect("Failed to load configuration");

    config.validate().expect("Invalid configuration");

    telemetry::init_telemetry(&config).expect("Failed to initialize telemetry");

    tracing::info!(
        "Starting {} on {}:{}",
        config.telemetry.service_name,
        config.server.host,
        config.server.port
    );

    let metrics = Arc::new(Metrics::new().expect("Failed to create metrics"));

    start_metrics_updater(metrics.clone()).await;

    let storage = PostgresStorageGateway::new(&config.database.url)
        .await
        .map_err(to_io_error)?;

    let migrator: Migrator = sqlx::migrate!("./migrations");

    storage.migrate(migrator).await.map_err(to_io_error)?;

    let auth = Authenticator::new(&config.jwt);
    let auth_arc = Arc::new(Authenticator::new(&config.jwt));
    let generator_secret_bytes: [u8; 32] =
        hex::decode(config.generator_secret.secret_key.as_bytes())
            .context("Cannot decode generator secret, not an hex strning")
            .map_err(to_io_error)?
            .try_into()
            .map_err(|_| anyhow!("Cannot convert to array of 32 bytes"))
            .map_err(to_io_error)?;

    let domain = web::Data::new(Domain::try_new(
        storage,
        auth,
        generator_secret_bytes,
        config.server.origin.clone(),
    ));

    let openapi = ApiDoc::openapi();

    let metrics_middleware = middleware_v1::MetricsMiddleware::new(metrics.clone());
    let jwt_middleware = middleware_v1::JwtMiddleware::new(auth_arc.clone());

    let server_host = config.server.host.clone();
    let server_port = config.server.port;
    let server_workers = config.server.workers;
    let server_keep_alive = config.server.keep_alive;
    let server_request_timeout = config.server.request_timeout;
    let jaeger_enabled = config.telemetry.jaeger_enabled;
    let jaeger_endpoint = config.telemetry.jaeger_endpoint.clone();
    let prometheus_enabled = config.metrics.prometheus_enabled;

    let server = HttpServer::new(move || {
        let cors = if config.server.host == "0.0.0.0" || config.server.host == "127.0.0.1" {
            Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600)
        } else {
            Cors::default()
                .allowed_origin(&config.server.origin)
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                .allowed_headers(vec![
                    actix_web::http::header::AUTHORIZATION,
                    actix_web::http::header::ACCEPT,
                    actix_web::http::header::CONTENT_TYPE,
                ])
                .max_age(3600)
        };

        App::new()
            .app_data(domain.to_owned())
            .app_data(web::Data::new((*metrics).clone()))
            .app_data(web::Data::new(config.clone()))
            .wrap(metrics_middleware.clone())
            .wrap(Logger::new(
                "%a %t \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T",
            ))
            .wrap(tracing_actix_web::TracingLogger::default())
            .wrap(cors)
            .service(handlers_v1::health)
            .service(handlers_v1::metrics_endpoint)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
            )
            .service(
                web::scope("/api/v1")
                    .service(handlers_v1::register)
                    .service(handlers_v1::login)
                    .service(web::scope("").wrap(jwt_middleware.clone())),
            )
            .default_service(web::route().to(|| async {
                actix_web::HttpResponse::NotFound().json(serde_json::json!({
                    "error": "not_found",
                    "message": "The requested resource was not found"
                }))
            }))
    })
    .workers(server_workers)
    .keep_alive(Duration::from_secs(server_keep_alive))
    .client_request_timeout(Duration::from_secs(server_request_timeout))
    .bind(format!("{server_host}:{server_port}"))?;

    tracing::info!(
        "🚀 Server running at http://{}:{}",
        server_host,
        server_port
    );
    tracing::info!(
        "📊 Metrics available at http://{}:{}/metrics",
        server_host,
        server_port
    );
    tracing::info!(
        "📚 Swagger UI available at http://{}:{}/swagger-ui/",
        server_host,
        server_port
    );

    if jaeger_enabled {
        tracing::info!("🔍 Tracing enabled with Jaeger at {}", jaeger_endpoint);
    }

    if prometheus_enabled {
        tracing::info!("📈 Prometheus metrics enabled at /metrics");
    }

    let result = server.run().await;

    result
}
