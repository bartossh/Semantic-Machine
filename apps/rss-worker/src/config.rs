use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{env, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssConfig {
    pub rss_urls: Vec<String>,
    pub interval: Duration,
    pub items_count: usize,
}

impl RssConfig {
    pub fn try_from_env() -> Result<Self> {
        let rss_urls = env::var("RSS_URLS")
            .context("RSS_URLS must be set")?
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let interval = Duration::from_secs(
            env::var("RSS_INTERVAL_SECONDS")
                .context("RSS_INTERVAL_SECONDS must be set")?
                .parse::<u64>()?,
        );

        let items_count = env::var("RSS_ITEMS_COUNT")
            .context("RSS_ITEMS_COUNT must be set")?
            .parse()
            .context("RSS_ITEMS_COUNT must be a valid number")?;

        Ok(Self {
            rss_urls,
            interval,
            items_count,
        })
    }
}
