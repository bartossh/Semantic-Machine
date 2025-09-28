use crate::config::RssConfig;
use anyhow::Result;
use reqwest::Client;
use rss::Channel;
use shared_states::RssItem;
use std::collections::HashSet;
use tokio::time::sleep;
use tracing::{error, info};

const INTERVAL: std::time::Duration = std::time::Duration::from_secs(60 * 10);
const ITEMS_PER_FEED: usize = 100;

/// Processor for RSS feeds.
pub struct Processor {
    visited: HashSet<String>,
}

impl Processor {
    /// Create a new instance of the processor.
    ///
    /// # Returns
    /// A new instance of the processor.
    pub fn new() -> Self {
        Self {
            visited: HashSet::new(),
        }
    }

    /// Run the processor.
    ///
    /// # Arguments
    /// * `config` - The configuration for the processor.
    ///
    /// # Returns
    /// A result indicating success or failure.
    pub async fn run(&mut self, config: &RssConfig) -> Result<()> {
        info!("Starting RSS worker for feeds: {:?}", config.rss_urls);
        loop {
            let mut repeated = false;
            'list: for url in config.rss_urls.iter() {
                info!("Starting fetching feeds from: [ {url} ]");
                let xml = match Client::new().get(url).send().await?.bytes().await {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        error!("Failed to fetch feed from [ {url} ]: {e}");
                        continue 'list;
                    }
                };
                let channel = match Channel::read_from(&xml[..]) {
                    Ok(channel) => channel,
                    Err(e) => {
                        error!("Failed to parse feed from [ {url} ]: {e}");
                        continue 'list;
                    }
                };

                info!("Feed: {}", channel.title());

                'items: for item in channel.items().iter().take(ITEMS_PER_FEED) {
                    let rss_item: RssItem = match item.try_into() {
                        Ok(item) => item,
                        Err(e) => {
                            error!("Failed to convert item [ {:?} ]: {e}", item);
                            continue 'items;
                        }
                    };

                    if self.update_if_not_contains(&rss_item) {
                        info!(
                            "Sending rss item with Nats with title {} and hash {}",
                            rss_item.title, rss_item.hash
                        );
                    } else {
                        repeated = true;
                    }
                }
            }

            if !repeated {
                self.reset_visited();
            }

            sleep(INTERVAL).await;
        }
    }

    fn update_if_not_contains(&mut self, item: &RssItem) -> bool {
        if self.visited.contains(&item.hash) {
            false
        } else {
            self.visited.insert(item.hash.clone());
            true
        }
    }

    fn reset_visited(&mut self) {
        self.visited.clear();
    }
}
