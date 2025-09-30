use crate::config::RssConfig;
use anyhow::{Result, anyhow};
use nats_middleware::NatsQueue;
use redis_middleware::RedisMiddleware;
use reqwest::Client;
use rss::Channel;
use shared_states::{RSS_QUEUE_NAME, RssItem};
use std::sync::Arc;
use tokio::{spawn, time::sleep};
use tracing::{error, info, warn};

/// Processor for RSS feeds.
pub struct Processor {
    queue: Arc<NatsQueue>,
    cache: Arc<RedisMiddleware>,
}

impl Processor {
    /// Create a new instance of the processor.
    ///
    /// # Returns
    /// A new instance of the processor.
    pub fn new(queue: Arc<NatsQueue>, cache: Arc<RedisMiddleware>) -> Self {
        Self { queue, cache }
    }

    /// Run the processor.
    ///
    /// # Arguments
    /// * `config` - The configuration for the processor.
    ///
    /// # Returns
    /// A result indicating success or failure.
    pub async fn run(&self, config: &RssConfig) -> Result<()> {
        info!("Starting RSS worker for feeds: {:?}", config.rss_urls);
        let items_count = config.items_count;

        loop {
            for url in config.rss_urls.iter() {
                let queue = self.queue.clone();
                let cache = self.cache.clone();
                let url = url.clone();
                spawn(async move {
                    match Self::process_url(queue, cache, url.clone(), items_count).await {
                        Ok(_) => (),
                        Err(e) => error!("Failed to process feed from ( {} ): {e}", url),
                    };
                });
            }

            sleep(config.interval).await;
        }
    }

    async fn process_url(
        queue: Arc<NatsQueue>,
        cache: Arc<RedisMiddleware>,
        url: String,
        items_count: usize,
    ) -> Result<()> {
        let xml = match Client::new().get(&url).send().await?.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                return Err(anyhow!("Failed to fetch feed from ( {url} ): {e}"));
            }
        };
        let channel = match Channel::read_from(&xml[..]) {
            Ok(channel) => channel,
            Err(e) => {
                return Err(anyhow!("Failed to parse feed from ( {url} ): {e}"));
            }
        };

        info!("Feed: {}", channel.title());

        for item in channel.items().iter().take(items_count) {
            let mut rss_item: RssItem = match item.try_into() {
                Ok(item) => item,
                Err(e) => {
                    error!("Failed to convert item [ {:?} ]: {e}", item);
                    continue;
                }
            };

            if match cache.retrieve(&rss_item.hash).await {
                Err(e) => {
                    error!("Cache connection faulure, {e}");
                    None
                }
                Ok(value) => value,
            }
            .is_some()
            {
                info!("RSS Item {} already processed", rss_item.hash);
                continue;
            }

            if let Err(e) = cache.store(&rss_item.hash, "").await {
                error!("Failed to store item in cache: {e}");
            }

            if let Err(e) = rss_item.extract_article_from_source().await {
                warn!(
                    "Failed to extract article from source for item [ {:?} ]: {e}",
                    item
                );
            }

            match queue.publish(RSS_QUEUE_NAME, &rss_item).await {
                Ok(_) => info!(
                    "Successfully sent rss item to NATs queue. Rss item title: ( {} ) and hash: ( {} )",
                    rss_item.title, rss_item.hash
                ),
                Err(e) => error!(
                    "Failed to send rss item to NATs queue. Rss item title: ( {} ) and hash: ( {} ). {e}",
                    rss_item.title, rss_item.hash
                ),
            };
        }
        Ok(())
    }
}
