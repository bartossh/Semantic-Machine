use anyhow::anyhow;
use nats_middleware::{NatsConfig, NatsQueue};
use std::{error::Error, sync::Arc};
use tracing::info;

mod config;
mod processor;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let worker_config = config::RssConfig::try_from_env().map_err(|e| anyhow!("{e}"))?;
    let nats_config = NatsConfig::from_env().map_err(|e| anyhow!("{e}"))?;
    let queue = NatsQueue::new(nats_config)
        .await
        .map_err(|e| anyhow!("{e}"))?;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    info!(
        "Starting RSS worker for feeds: {:?}",
        worker_config.rss_urls
    );

    let processor = processor::Processor::new(Arc::new(queue));
    processor.run(&worker_config).await?;

    Ok(())
}
