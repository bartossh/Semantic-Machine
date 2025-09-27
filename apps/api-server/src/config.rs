use nats_middleware::NatsConfig;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub jwt: JwtConfig,
    pub telemetry: TelemetryConfig,
    pub metrics: MetricsConfig,
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub nats: NatsConfig,
    pub minio: MinioConfig,
    pub generator_secret: GeneratorSecret,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub origin: String,
    pub workers: usize,
    pub max_connections: usize,
    pub keep_alive: u64,
    pub request_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
    pub issuer: String,
    pub audience: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub service_name: String,
    pub jaeger_enabled: bool,
    pub jaeger_endpoint: String,
    pub jaeger_sample_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub prometheus_enabled: bool,
    pub prometheus_endpoint: String,
    pub prometheus_port: u16,
    pub export_interval: u64,
    pub histogram_buckets: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
    pub enable_json: bool,
    pub enable_color: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub pool_size: u32,
    pub connection_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub host: String,
    pub port: u16,
    pub password: String,
    pub database: u8,
    pub pool_size: u32,
    pub connection_timeout: u64,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinioConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub region: String,
    pub use_ssl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorSecret {
    pub secret_key: String,
}

impl GeneratorSecret {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(GeneratorSecret {
            secret_key: env::var("GENERATOR_SECRET")
                .map_err(|_| ConfigError::MissingRequired("GENERATOR_SECRET".to_string()))?,
        })
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Config {
            server: ServerConfig::from_env()?,
            jwt: JwtConfig::from_env()?,
            telemetry: TelemetryConfig::from_env()?,
            metrics: MetricsConfig::from_env()?,
            logging: LoggingConfig::from_env()?,
            database: DatabaseConfig::from_env()?,
            redis: RedisConfig::from_env()?,
            nats: NatsConfig::from_env().map_err(|e| ConfigError::InvalidValue(e.to_string()))?,
            minio: MinioConfig::from_env()?,
            generator_secret: GeneratorSecret::from_env()?,
        })
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        // Add validation logic here
        if self.server.port == 0 {
            return Err(ConfigError::InvalidValue(
                "Server port cannot be 0".to_string(),
            ));
        }

        if self.jwt.secret.is_empty() {
            return Err(ConfigError::MissingRequired("JWT_SECRET".to_string()));
        }

        Ok(())
    }
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(ServerConfig {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|_| ConfigError::ParseError("SERVER_PORT".to_string()))?,
            origin: env::var("SERVER_ORIGIN").unwrap_or_else(|_| "*".to_string()),
            workers: env::var("SERVER_WORKERS")
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .map_err(|_| ConfigError::ParseError("SERVER_WORKERS".to_string()))?,
            max_connections: env::var("SERVER_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10000".to_string())
                .parse()
                .map_err(|_| ConfigError::ParseError("SERVER_MAX_CONNECTIONS".to_string()))?,
            keep_alive: env::var("SERVER_KEEP_ALIVE")
                .unwrap_or_else(|_| "75".to_string())
                .parse()
                .map_err(|_| ConfigError::ParseError("SERVER_KEEP_ALIVE".to_string()))?,
            request_timeout: env::var("SERVER_REQUEST_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .map_err(|_| ConfigError::ParseError("SERVER_REQUEST_TIMEOUT".to_string()))?,
        })
    }
}

impl JwtConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(JwtConfig {
            secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .map_err(|_| ConfigError::ParseError("JWT_EXPIRATION_HOURS".to_string()))?,
            issuer: env::var("JWT_ISSUER").unwrap_or_else(|_| "Semantic-Machine-api".to_string()),
            audience: env::var("JWT_AUDIENCE")
                .unwrap_or_else(|_| "Semantic-Machine-services".to_string()),
        })
    }
}

impl TelemetryConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(TelemetryConfig {
            enabled: env::var("TELEMETRY_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            service_name: env::var("TELEMETRY_SERVICE_NAME")
                .unwrap_or_else(|_| "api-service".to_string()),
            jaeger_enabled: env::var("JAEGER_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            jaeger_endpoint: env::var("JAEGER_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:14268/api/traces".to_string()),
            jaeger_sample_rate: env::var("JAEGER_SAMPLE_RATE")
                .unwrap_or_else(|_| "1.0".to_string())
                .parse()
                .unwrap_or(1.0),
        })
    }
}

impl MetricsConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let histogram_buckets = env::var("METRICS_HISTOGRAM_BUCKETS")
            .unwrap_or_else(|_| {
                "0.001,0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0".to_string()
            })
            .split(',')
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();

        Ok(MetricsConfig {
            enabled: env::var("METRICS_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            prometheus_enabled: env::var("PROMETHEUS_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            prometheus_endpoint: env::var("PROMETHEUS_ENDPOINT")
                .unwrap_or_else(|_| "/metrics".to_string()),
            prometheus_port: env::var("PROMETHEUS_PORT")
                .unwrap_or_else(|_| "9090".to_string())
                .parse()
                .unwrap_or(9090),
            export_interval: env::var("METRICS_EXPORT_INTERVAL")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            histogram_buckets,
        })
    }
}

impl LoggingConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(LoggingConfig {
            level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            format: env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string()),
            output: env::var("LOG_OUTPUT").unwrap_or_else(|_| "stdout".to_string()),
            enable_json: env::var("LOG_ENABLE_JSON")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_color: env::var("LOG_ENABLE_COLOR")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        })
    }
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let host = env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = env::var("DATABASE_PORT")
            .unwrap_or_else(|_| "5432".to_string())
            .parse()
            .unwrap_or(5432);
        let database =
            env::var("DATABASE_NAME").unwrap_or_else(|_| "semantic_machine_dev".to_string());
        let username = env::var("DATABASE_USER").unwrap_or_else(|_| "postgres".to_string());
        let password = env::var("DATABASE_PASSWORD").unwrap_or_else(|_| "password".to_string());

        let url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            format!("postgresql://{username}:{password}@{host}:{port}/{database}")
        });

        Ok(DatabaseConfig {
            url,
            host,
            port,
            database,
            username,
            password,
            pool_size: env::var("DATABASE_POOL_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            connection_timeout: env::var("DATABASE_CONNECTION_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            idle_timeout: env::var("DATABASE_IDLE_TIMEOUT")
                .unwrap_or_else(|_| "600".to_string())
                .parse()
                .unwrap_or(600),
            max_lifetime: env::var("DATABASE_MAX_LIFETIME")
                .unwrap_or_else(|_| "1800".to_string())
                .parse()
                .unwrap_or(1800),
        })
    }
}

impl RedisConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let host = env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = env::var("REDIS_PORT")
            .unwrap_or_else(|_| "6379".to_string())
            .parse()
            .unwrap_or(6379);
        let password = env::var("REDIS_PASSWORD").unwrap_or_else(|_| "password".to_string());

        let url =
            env::var("REDIS_URL").unwrap_or_else(|_| format!("redis://:{password}@{host}:{port}"));

        Ok(RedisConfig {
            url,
            host,
            port,
            password,
            database: env::var("REDIS_DATABASE")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap_or(0),
            pool_size: env::var("REDIS_POOL_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            connection_timeout: env::var("REDIS_CONNECTION_TIMEOUT")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            ttl_seconds: env::var("REDIS_TTL_SECONDS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .unwrap_or(3600),
        })
    }
}

impl MinioConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(MinioConfig {
            enabled: env::var("MINIO_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            endpoint: env::var("MINIO_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:9000".to_string()),
            access_key: env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_string()),
            secret_key: env::var("MINIO_SECRET_KEY")
                .unwrap_or_else(|_| "minioadmin123".to_string()),
            bucket: env::var("MINIO_BUCKET").unwrap_or_else(|_| "batobite-bucket".to_string()),
            region: env::var("MINIO_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            use_ssl: env::var("MINIO_USE_SSL")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingRequired(String),

    #[error("Failed to parse environment variable: {0}")]
    ParseError(String),

    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),
}
