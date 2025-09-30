use crate::telemetry::init_telemetry;
use anyhow::anyhow;
use nats_middleware::{NatsConfig, NatsQueue};
use redis_middleware::{Config as RedisConfig, RedisMiddleware};
use std::{error::Error, sync::Arc};
use tracing::info;

mod config;
mod processor;
mod telemetry;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    init_telemetry()?;

    let worker_config = config::RssConfig::try_from_env().map_err(|e| anyhow!("{e}"))?;
    let nats_config = NatsConfig::from_env().map_err(|e| anyhow!("{e}"))?;
    let redis_config = RedisConfig::from_env().map_err(|e| anyhow!("{e}"))?;
    let queue = NatsQueue::new(nats_config)
        .await
        .map_err(|e| anyhow!("{e}"))?;

    let redis_middleware = RedisMiddleware::new(&redis_config.redis_url)?;

    info!(
        "Starting RSS worker for feeds: {:?}",
        worker_config.rss_urls
    );

    let processor = processor::Processor::new(Arc::new(queue), Arc::new(redis_middleware));
    processor.run(&worker_config).await?;

    Ok(())
}
