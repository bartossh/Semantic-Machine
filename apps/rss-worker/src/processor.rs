use crate::config::RssConfig;
use anyhow::{Result, anyhow};
use nats_middleware::NatsQueue;
use reqwest::Client;
use rss::Channel;
use shared_states::{RSS_QUEUE_NAME, RssItem};
use std::sync::Arc;
use tokio::{spawn, time::sleep};
use tracing::{error, info};

/// Processor for RSS feeds.
pub struct Processor {
    queue: Arc<NatsQueue>,
}

impl Processor {
    /// Create a new instance of the processor.
    ///
    /// # Returns
    /// A new instance of the processor.
    pub fn new(queue: Arc<NatsQueue>) -> Self {
        Self { queue }
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
                let url = url.clone();
                spawn(async move {
                    match Self::process_url(queue, url.clone(), items_count).await {
                        Ok(_) => (),
                        Err(e) => error!("Failed to process feed from ( {} ): {e}", url),
                    };
                });
            }

            sleep(config.interval).await;
        }
    }

    async fn process_url(queue: Arc<NatsQueue>, url: String, items_count: usize) -> Result<()> {
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

            if let Err(e) = rss_item.update_article_from_source().await {
                error!(
                    "Failed to update article from source for item [ {:?} ]: {e}",
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
