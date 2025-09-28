use std::error::Error;
use tracing::info;

mod config;
mod processor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let config = config::RssConfig::try_from_env().map_err(|e| format!("{e}"))?;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    info!("Starting RSS worker for feeds: {:?}", config.rss_urls);

    let mut processor = processor::Processor::new();
    processor.run(&config).await?;

    Ok(())
}
