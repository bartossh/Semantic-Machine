use async_nats::{Client, ConnectOptions, Message};
use serde::{Deserialize, Serialize};
use std::{env, time::Duration};
use thiserror::Error;
use tokio::time::timeout;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum NatsError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Timeout error: operation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Configuration error: env {0} not found")]
    Configuration(String),

    #[error("Subject error: {0}")]
    Subject(String),
}

pub type NatsResult<T> = Result<T, NatsError>;

/// NATS queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    /// NATS server URL
    pub url: String,

    /// Client name for connection identification
    pub client_name: String,

    /// Maximum reconnection attempts
    pub max_reconnects: usize,

    /// Connection timeout in milliseconds
    pub connect_timeout_ms: u64,

    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,

    /// Enable TLS
    pub tls_enabled: bool,

    /// Authentication token
    pub auth_token: Option<String>,
}

impl NatsConfig {
    pub fn from_env() -> Result<Self, NatsError> {
        let url = env::var("NATS_URL")
            .map_err(|e| NatsError::Configuration(format!("NATS_URL, {e:?}")))?;
        let client_name = env::var("NATS_CLIENT_NAME")
            .map_err(|e| NatsError::Configuration(format!("NATS_CLIENT_NAME, {e:?}")))?;
        let max_reconnects = env::var("NATS_MAX_RECONNECTS")
            .map_err(|e| NatsError::Configuration(format!("NATS_MAX_RECONNECTS, {e:?}")))?
            .parse()
            .map_err(|e| NatsError::Configuration(format!("NATS_MAX_RECONNECTS, {e:?}")))?;
        let connect_timeout_ms = env::var("NATS_CONNECT_TIMEOUT_MS")
            .map_err(|e| NatsError::Configuration(format!("NATS_CONNECT_TIMEOUT_MS, {e:?}")))?
            .parse()
            .map_err(|e| NatsError::Configuration(format!("NATS_CONNECT_TIMEOUT_MS, {e:?}")))?;
        let request_timeout_ms = env::var("NATS_REQUEST_TIMEOUT_MS")
            .map_err(|e| NatsError::Configuration(format!("NATS_REQUEST_TIMEOUT_MS, {e:?}")))?
            .parse()
            .map_err(|e| NatsError::Configuration(format!("NATS_REQUEST_TIMEOUT_MS, {e:?}")))?;
        let tls_enabled = env::var("NATS_TLS_ENABLED")
            .map_err(|e| NatsError::Configuration(format!("NATS_TLS_ENABLED, {e:?}")))?
            .parse()
            .map_err(|e| NatsError::Configuration(format!("NATS_TLS_ENABLED, {e:?}")))?;
        let auth_token = env::var("NATS_AUTH_TOKEN").ok();

        Ok(Self {
            url,
            client_name,
            max_reconnects,
            connect_timeout_ms,
            request_timeout_ms,
            tls_enabled,
            auth_token,
        })
    }
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".to_string(),
            client_name: "webhook-events".to_string(),
            max_reconnects: 10,
            connect_timeout_ms: 5000,
            request_timeout_ms: 30000,
            tls_enabled: false,
            auth_token: None,
        }
    }
}

/// NATS queue client for webhook events
#[derive(Debug, Clone)]
pub struct NatsQueue {
    client: Client,
    config: NatsConfig,
}

impl NatsQueue {
    /// Create a new NATS queue connection
    ///
    /// # Arguments
    /// * `config` - Configuration for the NATS queue connection
    ///
    /// # Returns
    /// * `NatsResult<Self>` - Result of the connection attempt
    pub async fn new(config: NatsConfig) -> NatsResult<Self> {
        let mut connect_opts = ConnectOptions::new()
            .name(&config.client_name)
            .connection_timeout(Duration::from_millis(config.connect_timeout_ms));

        if let Some(token) = &config.auth_token {
            connect_opts = connect_opts.token(token.clone());
        }

        if config.tls_enabled {
            connect_opts = connect_opts.require_tls(true);
        }

        info!(
            url = %config.url,
            client_name = %config.client_name,
            "Connecting to NATS server"
        );

        let client = async_nats::connect_with_options(&config.url, connect_opts)
            .await
            .map_err(|e| NatsError::Connection(e.to_string()))?;

        info!("Successfully connected to NATS server");

        Ok(Self { client, config })
    }

    /// Publish a message to a subject
    ///
    /// # Arguments
    /// * `subject` - The subject to publish the message to
    /// * `payload` - The payload to publish
    ///
    /// # Returns
    /// * `NatsResult<()>` - Result of the publish attempt
    pub async fn publish<T>(&self, subject: &str, payload: &T) -> NatsResult<()>
    where
        T: Serialize,
    {
        let data = serde_json::to_vec(payload)?;

        self.client
            .publish(subject.to_string(), data.into())
            .await
            .map_err(|e| NatsError::Connection(e.to_string()))?;

        Ok(())
    }

    /// Publish a message with a reply subject
    ///
    /// # Arguments
    /// * `subject` - The subject to publish the message to
    /// * `reply` - The reply subject to use
    /// * `payload` - The payload to publish
    ///
    /// # Returns
    /// * `NatsResult<()>` - Result of the publish attempt
    pub async fn publish_with_reply<T>(
        &self,
        subject: &str,
        reply: &str,
        payload: &T,
    ) -> NatsResult<()>
    where
        T: Serialize,
    {
        let data = serde_json::to_vec(payload)?;

        self.client
            .publish_with_reply(subject.to_string(), reply.to_string(), data.into())
            .await
            .map_err(|e| NatsError::Connection(e.to_string()))?;

        Ok(())
    }

    /// Subscribe to a subject
    ///
    /// # Arguments
    /// * `subject` - The subject to subscribe to
    ///
    /// # Returns
    /// * `NatsResult<async_nats::Subscriber>` - Result of the subscription attempt
    pub async fn subscribe(&self, subject: &str) -> NatsResult<async_nats::Subscriber> {
        let subscriber = self
            .client
            .subscribe(subject.to_string())
            .await
            .map_err(|e| NatsError::Connection(e.to_string()))?;

        info!(subject = %subject, "Subscribed to NATS subject");

        Ok(subscriber)
    }

    /// Subscribe to a subject with a queue group
    ///
    /// # Arguments
    /// * `subject` - The subject to subscribe to
    /// * `queue` - The queue group to subscribe to
    ///
    /// # Returns
    /// * `NatsResult<async_nats::Subscriber>` - Result of the subscription attempt
    pub async fn queue_subscribe(
        &self,
        subject: &str,
        queue: &str,
    ) -> NatsResult<async_nats::Subscriber> {
        let subscriber = self
            .client
            .queue_subscribe(subject.to_string(), queue.to_string())
            .await
            .map_err(|e| NatsError::Connection(e.to_string()))?;

        info!(
            subject = %subject,
            queue = %queue,
            "Subscribed to NATS subject with queue group"
        );

        Ok(subscriber)
    }

    /// Make a request and wait for a response
    ///
    /// # Arguments
    /// * `subject` - The subject to make the request to
    /// * `payload` - The payload to send with the request
    ///
    /// # Returns
    /// * `NatsResult<R>` - Result of the request attempt
    pub async fn request<T, R>(&self, subject: &str, payload: &T) -> NatsResult<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let data = serde_json::to_vec(payload)?;

        let response = timeout(
            Duration::from_millis(self.config.request_timeout_ms),
            self.client.request(subject.to_string(), data.into()),
        )
        .await
        .map_err(|_| NatsError::Timeout {
            timeout_ms: self.config.request_timeout_ms,
        })?
        .map_err(|e| NatsError::Connection(e.to_string()))?;

        let result = serde_json::from_slice(&response.payload)?;
        Ok(result)
    }

    /// Deserialize a NATS message payload
    ///
    /// # Arguments
    /// * `message` - The message to deserialize
    ///
    /// # Returns
    /// * `NatsResult<T>` - Result of the deserialization attempt
    pub fn deserialize_message<T>(&self, message: &Message) -> NatsResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let result = serde_json::from_slice(&message.payload)?;
        Ok(result)
    }

    /// Reply to a message
    ///
    /// # Arguments
    /// * `message` - The message to reply to
    /// * `payload` - The payload to send with the reply
    ///
    /// # Returns
    /// * `NatsResult<()>` - Result of the reply attempt
    pub async fn reply<T>(&self, message: &Message, payload: &T) -> NatsResult<()>
    where
        T: Serialize,
    {
        if let Some(reply_subject) = &message.reply {
            let data = serde_json::to_vec(payload)?;
            self.client
                .publish(reply_subject.clone(), data.into())
                .await
                .map_err(|e| NatsError::Connection(e.to_string()))?;
        } else {
            return Err(NatsError::Subject(
                "Message does not have a reply subject".to_string(),
            ));
        }

        Ok(())
    }

    /// Get connection statistics
    ///
    /// # Returns
    /// * `ConnectionStatus` - Connection status information
    pub fn connection_status(&self) -> ConnectionStatus {
        ConnectionStatus {
            is_connected: true, // Simplified - async-nats doesn't have is_closed method
            server_info: self.client.server_info().clone(),
        }
    }

    /// Flush pending messages
    ///
    /// # Returns
    /// * `NatsResult<()>` - Result of the flush operation
    pub async fn flush(&self) -> NatsResult<()> {
        self.client
            .flush()
            .await
            .map_err(|e| NatsError::Connection(e.to_string()))?;
        Ok(())
    }
}

/// Connection status information
#[derive(Debug, Clone)]
pub struct ConnectionStatus {
    pub is_connected: bool,
    pub server_info: async_nats::ServerInfo,
}

/// Webhook event for NATS messaging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEventMessage {
    pub event_id: uuid::Uuid,
    pub event_type: String,
    pub source: String,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl WebhookEventMessage {
    pub fn new(
        event_id: uuid::Uuid,
        event_type: String,
        source: String,
        data: serde_json::Value,
    ) -> Self {
        Self {
            event_id,
            event_type,
            source,
            data,
            timestamp: chrono::Utc::now(),
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    pub fn should_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }
}

/// Subject builder for consistent naming
pub struct SubjectBuilder {
    prefix: String,
}

impl SubjectBuilder {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    pub fn webhook_received(&self) -> String {
        format!("{}.webhook.received", self.prefix)
    }

    pub fn webhook_processed(&self) -> String {
        format!("{}.webhook.processed", self.prefix)
    }

    pub fn webhook_failed(&self) -> String {
        format!("{}.webhook.failed", self.prefix)
    }

    pub fn webhook_retry(&self) -> String {
        format!("{}.webhook.retry", self.prefix)
    }

    pub fn health_check(&self) -> String {
        format!("{}.health", self.prefix)
    }

    pub fn custom(&self, suffix: &str) -> String {
        format!("{}.{}", self.prefix, suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_webhook_event_message() {
        let event_id = Uuid::new_v4();
        let mut message = WebhookEventMessage::new(
            event_id,
            "user.created".to_string(),
            "api.example.com".to_string(),
            serde_json::json!({"user_id": 123}),
        );

        assert_eq!(message.event_id, event_id);
        assert_eq!(message.retry_count, 0);
        assert!(message.should_retry());

        message.increment_retry();
        assert_eq!(message.retry_count, 1);
        assert!(message.should_retry());

        // Exceed max retries
        message.retry_count = 5;
        assert!(!message.should_retry());
    }

    #[test]
    fn test_subject_builder() {
        let builder = SubjectBuilder::new("semantic_machine.webhooks");

        assert_eq!(builder.webhook_received(), "semantic_machine.webhooks.webhook.received");
        assert_eq!(
            builder.webhook_processed(),
            "semantic_machine.webhooks.webhook.processed"
        );
        assert_eq!(builder.webhook_failed(), "semantic_machine.webhooks.webhook.failed");
        assert_eq!(builder.health_check(), "semantic_machine.webhooks.health");
        assert_eq!(builder.custom("test"), "semantic_machine.webhooks.test");
    }

    #[test]
    fn test_nats_config_default() {
        let config = NatsConfig::default();
        assert_eq!(config.url, "nats://localhost:4222");
        assert_eq!(config.client_name, "webhook-events");
        assert_eq!(config.connect_timeout_ms, 5000);
        assert!(!config.tls_enabled);
    }

    #[tokio::test]
    async fn test_serialization() {
        let message = WebhookEventMessage::new(
            Uuid::new_v4(),
            "test.event".to_string(),
            "test.source".to_string(),
            serde_json::json!({"test": "data"}),
        );

        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: WebhookEventMessage = serde_json::from_str(&serialized).unwrap();

        assert_eq!(message.event_id, deserialized.event_id);
        assert_eq!(message.event_type, deserialized.event_type);
    }
}
