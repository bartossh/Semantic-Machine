use crate::config::{Config, TelemetryConfig};
use opentelemetry::global;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use prometheus::{
    Encoder, Gauge, GaugeVec, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, IntGaugeVec,
    Opts, Registry, TextEncoder,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry as TracingRegistry};

/// Main metrics structure containing all Prometheus metrics
#[derive(Clone)]
#[allow(dead_code)]
pub struct Metrics {
    pub registry: Registry,

    // HTTP Metrics
    pub http_requests_total: IntCounterVec,
    pub http_request_duration: HistogramVec,
    pub http_request_size: HistogramVec,
    pub http_response_size: HistogramVec,
    pub http_active_requests: IntGauge,

    // Connection Metrics
    pub active_connections: IntGauge,
    pub connection_errors: IntCounterVec,

    // Authentication Metrics
    pub auth_attempts: IntCounterVec,
    pub auth_failures: IntCounterVec,
    pub active_sessions: IntGauge,
    pub jwt_validations: IntCounterVec,

    // Business Metrics
    pub user_registrations: IntCounterVec,
    pub user_logins: IntCounterVec,
    pub api_calls_by_endpoint: IntCounterVec,
    pub api_errors_by_type: IntCounterVec,

    // System Metrics
    pub memory_usage: GaugeVec,
    pub cpu_usage: Gauge,
    pub thread_count: IntGauge,
    pub uptime_seconds: IntGauge,

    // Database Metrics
    pub db_connections_active: IntGauge,
    pub db_connections_idle: IntGauge,
    pub db_query_duration: HistogramVec,
    pub db_errors: IntCounterVec,

    // Cache Metrics
    pub cache_hits: IntCounterVec,
    pub cache_misses: IntCounterVec,
    pub cache_operations: IntCounterVec,
    pub cache_size: IntGaugeVec,

    // Rate Limiting Metrics
    pub rate_limit_hits: IntCounterVec,
    pub rate_limit_exceeded: IntCounterVec,

    // Custom Business Metrics
    pub api_version_usage: IntCounterVec,
    pub feature_usage: IntCounterVec,
    pub webhook_deliveries: IntCounterVec,
    pub webhook_failures: IntCounterVec,
}

#[allow(dead_code)]
impl Metrics {
    /// Create a new Metrics instance with all configured Prometheus metrics
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let http_requests_total = IntCounterVec::new(
            Opts::new(
                "api_http_requests_total",
                "Total number of HTTP requests received",
            ),
            &["method", "endpoint", "status", "version"],
        )?;

        let http_request_duration = HistogramVec::new(
            HistogramOpts::new(
                "api_http_request_duration_seconds",
                "HTTP request latency in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["method", "endpoint", "status"],
        )?;

        let http_request_size = HistogramVec::new(
            HistogramOpts::new(
                "api_http_request_size_bytes",
                "HTTP request body size in bytes",
            )
            .buckets(vec![
                100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0,
            ]),
            &["method", "endpoint"],
        )?;

        let http_response_size = HistogramVec::new(
            HistogramOpts::new(
                "api_http_response_size_bytes",
                "HTTP response body size in bytes",
            )
            .buckets(vec![
                100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0,
            ]),
            &["method", "endpoint", "status"],
        )?;

        let http_active_requests =
            IntGauge::new("api_http_active_requests", "Number of active HTTP requests")?;

        let active_connections = IntGauge::new(
            "api_connections_active",
            "Number of active client connections",
        )?;

        let connection_errors = IntCounterVec::new(
            Opts::new(
                "api_connections_errors_total",
                "Total number of connection errors",
            ),
            &["error_type"],
        )?;

        let auth_attempts = IntCounterVec::new(
            Opts::new(
                "api_auth_attempts_total",
                "Total number of authentication attempts",
            ),
            &["method", "status"],
        )?;

        let auth_failures = IntCounterVec::new(
            Opts::new(
                "api_auth_failures_total",
                "Total number of authentication failures",
            ),
            &["reason"],
        )?;

        let active_sessions =
            IntGauge::new("api_auth_active_sessions", "Number of active user sessions")?;

        let jwt_validations = IntCounterVec::new(
            Opts::new(
                "api_auth_jwt_validations_total",
                "Total number of JWT validations",
            ),
            &["status"],
        )?;

        let user_registrations = IntCounterVec::new(
            Opts::new(
                "api_users_registrations_total",
                "Total number of user registrations",
            ),
            &["status"],
        )?;

        let user_logins = IntCounterVec::new(
            Opts::new("api_users_logins_total", "Total number of user logins"),
            &["status"],
        )?;

        let api_calls_by_endpoint = IntCounterVec::new(
            Opts::new("api_requests_calls_total", "Total API calls by endpoint"),
            &["endpoint", "method", "version"],
        )?;

        let api_errors_by_type = IntCounterVec::new(
            Opts::new("api_errors_total", "Total API errors by type"),
            &["error_type", "endpoint"],
        )?;

        let memory_usage = GaugeVec::new(
            Opts::new("api_system_memory_usage_bytes", "Memory usage in bytes"),
            &["type"],
        )?;

        let cpu_usage = Gauge::new("api_system_cpu_usage_percent", "CPU usage percentage")?;

        let thread_count = IntGauge::new("api_system_thread_count", "Number of active threads")?;

        let uptime_seconds =
            IntGauge::new("api_system_uptime_seconds", "Service uptime in seconds")?;

        let db_connections_active = IntGauge::new(
            "api_database_connections_active",
            "Number of active database connections",
        )?;

        let db_connections_idle = IntGauge::new(
            "api_database_connections_idle",
            "Number of idle database connections",
        )?;

        let db_query_duration = HistogramVec::new(
            HistogramOpts::new(
                "api_database_query_duration_seconds",
                "Database query duration in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
            ]),
            &["operation", "table"],
        )?;

        let db_errors = IntCounterVec::new(
            Opts::new(
                "api_database_errors_total",
                "Total number of database errors",
            ),
            &["error_type", "operation"],
        )?;

        let cache_hits = IntCounterVec::new(
            Opts::new("api_cache_hits_total", "Total number of cache hits"),
            &["cache_name"],
        )?;

        let cache_misses = IntCounterVec::new(
            Opts::new("api_cache_misses_total", "Total number of cache misses"),
            &["cache_name"],
        )?;

        let cache_operations = IntCounterVec::new(
            Opts::new("api_cache_operations_total", "Total cache operations"),
            &["operation", "cache_name"],
        )?;

        let cache_size = IntGaugeVec::new(
            Opts::new("api_cache_size_entries", "Number of entries in cache"),
            &["cache_name"],
        )?;

        let rate_limit_hits = IntCounterVec::new(
            Opts::new("api_rate_limit_hits_total", "Total rate limit checks"),
            &["endpoint", "client_type"],
        )?;

        let rate_limit_exceeded = IntCounterVec::new(
            Opts::new(
                "api_rate_limit_exceeded_total",
                "Total rate limit exceeded events",
            ),
            &["endpoint", "client_type"],
        )?;

        let api_version_usage = IntCounterVec::new(
            Opts::new("api_versioning_usage_total", "API version usage"),
            &["version", "endpoint"],
        )?;

        let feature_usage = IntCounterVec::new(
            Opts::new("api_features_usage_total", "Feature usage tracking"),
            &["feature_name", "user_type"],
        )?;

        let webhook_deliveries = IntCounterVec::new(
            Opts::new("api_webhooks_deliveries_total", "Total webhook deliveries"),
            &["event_type", "status"],
        )?;

        let webhook_failures = IntCounterVec::new(
            Opts::new(
                "api_webhooks_failures_total",
                "Total webhook delivery failures",
            ),
            &["event_type", "failure_reason"],
        )?;

        registry.register(Box::new(http_requests_total.clone()))?;
        registry.register(Box::new(http_request_duration.clone()))?;
        registry.register(Box::new(http_request_size.clone()))?;
        registry.register(Box::new(http_response_size.clone()))?;
        registry.register(Box::new(http_active_requests.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(connection_errors.clone()))?;
        registry.register(Box::new(auth_attempts.clone()))?;
        registry.register(Box::new(auth_failures.clone()))?;
        registry.register(Box::new(active_sessions.clone()))?;
        registry.register(Box::new(jwt_validations.clone()))?;
        registry.register(Box::new(user_registrations.clone()))?;
        registry.register(Box::new(user_logins.clone()))?;
        registry.register(Box::new(api_calls_by_endpoint.clone()))?;
        registry.register(Box::new(api_errors_by_type.clone()))?;
        registry.register(Box::new(memory_usage.clone()))?;
        registry.register(Box::new(cpu_usage.clone()))?;
        registry.register(Box::new(thread_count.clone()))?;
        registry.register(Box::new(uptime_seconds.clone()))?;
        registry.register(Box::new(db_connections_active.clone()))?;
        registry.register(Box::new(db_connections_idle.clone()))?;
        registry.register(Box::new(db_query_duration.clone()))?;
        registry.register(Box::new(db_errors.clone()))?;
        registry.register(Box::new(cache_hits.clone()))?;
        registry.register(Box::new(cache_misses.clone()))?;
        registry.register(Box::new(cache_operations.clone()))?;
        registry.register(Box::new(cache_size.clone()))?;
        registry.register(Box::new(rate_limit_hits.clone()))?;
        registry.register(Box::new(rate_limit_exceeded.clone()))?;
        registry.register(Box::new(api_version_usage.clone()))?;
        registry.register(Box::new(feature_usage.clone()))?;
        registry.register(Box::new(webhook_deliveries.clone()))?;
        registry.register(Box::new(webhook_failures.clone()))?;

        Ok(Self {
            registry,
            http_requests_total,
            http_request_duration,
            http_request_size,
            http_response_size,
            http_active_requests,
            active_connections,
            connection_errors,
            auth_attempts,
            auth_failures,
            active_sessions,
            jwt_validations,
            user_registrations,
            user_logins,
            api_calls_by_endpoint,
            api_errors_by_type,
            memory_usage,
            cpu_usage,
            thread_count,
            uptime_seconds,
            db_connections_active,
            db_connections_idle,
            db_query_duration,
            db_errors,
            cache_hits,
            cache_misses,
            cache_operations,
            cache_size,
            rate_limit_hits,
            rate_limit_exceeded,
            api_version_usage,
            feature_usage,
            webhook_deliveries,
            webhook_failures,
        })
    }

    #[inline(always)]
    pub fn export(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }

    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn record_http_request(
        &self,
        method: &str,
        endpoint: &str,
        status: u16,
        duration: f64,
        request_size: usize,
        response_size: usize,
        version: &str,
    ) {
        let status_str = status.to_string();

        self.http_requests_total
            .with_label_values(&[method, endpoint, &status_str, version])
            .inc();

        self.http_request_duration
            .with_label_values(&[method, endpoint, &status_str])
            .observe(duration);

        self.http_request_size
            .with_label_values(&[method, endpoint])
            .observe(request_size as f64);

        self.http_response_size
            .with_label_values(&[method, endpoint, &status_str])
            .observe(response_size as f64);

        self.api_calls_by_endpoint
            .with_label_values(&[endpoint, method, version])
            .inc();
    }

    #[inline(always)]
    pub fn record_auth_attempt(&self, method: &str, success: bool) {
        let status = if success { "success" } else { "failure" };
        self.auth_attempts
            .with_label_values(&[method, status])
            .inc();

        if !success {
            self.auth_failures.with_label_values(&[method]).inc();
        }
    }

    #[inline(always)]
    pub fn record_user_registration(&self, success: bool) {
        let status = if success { "success" } else { "failure" };
        self.user_registrations.with_label_values(&[status]).inc();
    }

    #[inline(always)]
    pub fn record_user_login(&self, success: bool) {
        let status = if success { "success" } else { "failure" };
        self.user_logins.with_label_values(&[status]).inc();
    }

    #[inline(always)]
    pub fn record_jwt_validation(&self, valid: bool) {
        let status = if valid { "valid" } else { "invalid" };
        self.jwt_validations.with_label_values(&[status]).inc();
    }

    #[inline(always)]
    pub fn record_db_query(&self, operation: &str, table: &str, duration: f64) {
        self.db_query_duration
            .with_label_values(&[operation, table])
            .observe(duration);
    }

    #[inline(always)]
    pub fn record_db_error(&self, error_type: &str, operation: &str) {
        self.db_errors
            .with_label_values(&[error_type, operation])
            .inc();
    }

    #[inline(always)]
    pub fn record_cache_hit(&self, cache_name: &str) {
        self.cache_hits.with_label_values(&[cache_name]).inc();
    }

    #[inline(always)]
    pub fn record_cache_miss(&self, cache_name: &str) {
        self.cache_misses.with_label_values(&[cache_name]).inc();
    }

    #[inline(always)]
    pub fn record_cache_operation(&self, operation: &str, cache_name: &str) {
        self.cache_operations
            .with_label_values(&[operation, cache_name])
            .inc();
    }

    #[inline(always)]
    pub fn update_system_metrics(&self) {
        let start_time = std::env::var("PROCESS_START_TIME")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or_else(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            });

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.uptime_seconds.set((now - start_time) as i64);

        self.memory_usage.with_label_values(&["used"]).set(0.0);
        self.memory_usage.with_label_values(&["free"]).set(0.0);
        self.cpu_usage.set(0.0);
        self.thread_count.set(0);
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics")
    }
}

/// Initialize OpenTelemetry tracer (simplified for compatibility)
pub fn init_tracer(config: &TelemetryConfig) -> bool {
    if !config.enabled || !config.jaeger_enabled {
        return false;
    }

    global::set_text_map_propagator(TraceContextPropagator::new());

    tracing::info!("Telemetry configured for service: {}", config.service_name);

    true
}

/// Initialize telemetry with tracing and metrics
pub fn init_telemetry(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.logging.level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    init_tracer(&config.telemetry);

    let subscriber = TracingRegistry::default().with(env_filter);

    if config.logging.enable_json {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .with_current_span(true)
            .with_span_list(true);

        let subscriber = subscriber.with(fmt_layer);
        tracing::subscriber::set_global_default(subscriber)?;
    } else {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(config.logging.enable_color);

        let subscriber = subscriber.with(fmt_layer);
        tracing::subscriber::set_global_default(subscriber)?;
    }

    tracing::info!(
        "Telemetry initialized with level: {}, format: {}",
        config.logging.level,
        config.logging.format
    );

    Ok(())
}

/// Helper to create a span for database operations
#[macro_export]
macro_rules! db_span {
    ($op:expr, $table:expr) => {
        tracing::info_span!(
            "db_query",
            otel.name = format!("{} {}", $op, $table),
            db.operation = $op,
            db.table = $table,
        )
    };
}

/// Helper to create a span for cache operations
#[macro_export]
macro_rules! cache_span {
    ($op:expr, $key:expr) => {
        tracing::info_span!(
            "cache_operation",
            otel.name = format!("cache.{}", $op),
            cache.operation = $op,
            cache.key = $key,
        )
    };
}

/// Helper to create a span for external API calls
#[macro_export]
macro_rules! api_span {
    ($method:expr, $url:expr) => {
        tracing::info_span!(
            "api_call",
            otel.name = format!("{} {}", $method, $url),
            http.method = $method,
            http.url = $url,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new().unwrap();
        assert!(metrics.export().is_ok());
    }

    #[test]
    fn test_metric_recording() {
        let metrics = Metrics::new().unwrap();

        metrics.record_http_request("GET", "/api/v1/health", 200, 0.1, 0, 100, "v1");
        metrics.record_auth_attempt("login", true);
        metrics.record_user_registration(true);
        metrics.record_user_login(true);
        metrics.record_jwt_validation(true);
        metrics.record_cache_hit("session");
        metrics.record_cache_miss("user");

        let export = metrics.export().unwrap();
        assert!(export.contains("http_requests_total"));
        assert!(export.contains("auth_attempts_total"));
    }
}
