use chrono::{DateTime, Utc};
use rss::Item;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::prelude::FromRow;

/// Represents an RSS feed with relation to rss items hash.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq, Hash)]
pub struct RssFeed {
    pub id: Option<u64>,
    pub title: String,
    pub rss_item_hash: String,
}

/// RssItem represents an item in an RSS feed.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq, Hash)]
pub struct RssItem {
    pub hash: String,
    pub title: String,
    pub link: String,
    pub description: String,
    pub published_timestamp: u64,
    pub fetched_timestamp: u64,
    pub comments_url: Option<String>,
    pub category: String,
    pub author: String,
}

impl TryFrom<&Item> for RssItem {
    type Error = anyhow::Error;

    fn try_from(item: &Item) -> Result<Self, Self::Error> {
        let dt = DateTime::parse_from_rfc2822(item.pub_date().unwrap_or_default())?;
        let dt_utc = dt.with_timezone(&Utc);
        let published_timestamp = dt_utc.timestamp_millis() as u64;
        let fetched_timestamp = Utc::now().timestamp_millis() as u64;
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
            comments_url: item.comments().map(|s| s.to_string()),
            category: item
                .categories()
                .iter()
                .map(|c| c.name().to_string())
                .collect::<Vec<String>>()
                .join(", "),
            author: item.author().unwrap_or_default().to_string(),
        })
    }
}
