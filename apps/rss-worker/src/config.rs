use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssConfig {
    pub rss_urls: Vec<String>,
}

impl RssConfig {
    pub fn try_from_env() -> Result<Self> {
        let rss_urls = env::var("RSS_URLS")
            .context("RSS_URLS must be set")?
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(Self { rss_urls })
    }
}
