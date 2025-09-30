use crate::extract_article;
use chrono::{DateTime, Utc};
use rss::Item;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::prelude::FromRow;

pub const RSS_QUEUE_NAME: &str = "rss_items";

/// RssItem represents an item in an RSS feed.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq, Hash)]
pub struct RssItem {
    pub hash: String,
    pub title: String,
    pub link: String,
    pub description: String,
    pub published_timestamp: i64,
    pub fetched_timestamp: i64,
    pub comments_url: String,
    pub category: String,
    pub author: String,
    pub article: String,
}

impl RssItem {
    pub async fn extract_article_from_source(&mut self) -> anyhow::Result<()> {
        self.article = extract_article(&self.link).await?;
        Ok(())
    }
}

impl TryFrom<&Item> for RssItem {
    type Error = anyhow::Error;

    fn try_from(item: &Item) -> Result<Self, Self::Error> {
        let dt = DateTime::parse_from_rfc2822(item.pub_date().unwrap_or_default())?;
        let dt_utc = dt.with_timezone(&Utc);
        let published_timestamp = dt_utc.timestamp_millis();
        let fetched_timestamp = Utc::now().timestamp_millis();
        let mut hasher = Sha256::new();
        hasher.update(item.title().unwrap_or_default().as_bytes());
        hasher.update(item.author().unwrap_or_default().as_bytes());
        hasher.update(item.link().unwrap_or_default().as_bytes());
        hasher.update(item.description().unwrap_or_default().as_bytes());
        hasher.update(item.pub_date().unwrap_or_default().as_bytes());
        let result = hasher.finalize();
        let hash = hex::encode(result);

        Ok(RssItem {
            hash,
            title: item.title().unwrap_or_default().to_string(),
            link: item.link().unwrap_or_default().to_string(),
            description: item.description().unwrap_or_default().to_string(),
            published_timestamp,
            fetched_timestamp,
            comments_url: item.comments().unwrap_or_default().to_string(),
            category: item
                .categories()
                .iter()
                .map(|c| c.name().to_string())
                .collect::<Vec<String>>()
                .join(", "),
            author: item.author().unwrap_or_default().to_string(),
            article: String::new(),
        })
    }
}
