use crate::{
    database::{PostgresStorageGateway, StoreInsertBulk, StoreReadBulkEntities},
    impl_read_bulk_by_ids, impl_store_bulk,
};
use anyhow::{Result, anyhow};
use futures::StreamExt;
use nats_middleware::NatsQueue;
use shared_states::{RSS_QUEUE_NAME, RssItem};
use sqlx::{Arguments, Row, postgres::PgArguments};

impl_store_bulk!(
    RssItem,
    String,
    "rss_items",
    [
        hash,
        title,
        link,
        description,
        published_timestamp,
        fetched_timestamp,
        comments_url,
        category,
        author,
        article
    ],
    "hash",
);

impl_read_bulk_by_ids!(
    RssItem,
    String,
    "rss_items",
    [
        hash,
        title,
        link,
        description,
        published_timestamp,
        fetched_timestamp,
        comments_url,
        category,
        author,
        article
    ],
    "hash",
);

pub struct RssFeedsProcessor {
    storage: PostgresStorageGateway,
    queue: NatsQueue,
}

impl RssFeedsProcessor {
    pub fn new(storage: PostgresStorageGateway, queue: NatsQueue) -> Self {
        Self { storage, queue }
    }

    /// Run the processor reading messages from the queue and saving them to the database.
    pub async fn run(&self) -> Result<()> {
        let mut channel = self.queue.subscribe(RSS_QUEUE_NAME).await?;

        while let Some(message) = channel.next().await {
            let rss_item: RssItem = serde_json::from_slice(&message.payload)?;
            let hash = rss_item.hash.clone();
            match self.storage.read_bulk_by_ids(&[hash]).await {
                Ok(ids) => {
                    if !ids.is_empty() {
                        tracing::info!("RSS item already exists: {ids:?}");
                        continue;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to read RSS item: {}", e);
                    continue;
                }
            }
            match self.storage.insert_bulk(&[rss_item]).await {
                Ok(hash) => tracing::info!("Successfully inserted RSS item: {hash:?}"),
                Err(e) => {
                    tracing::error!("Failed to insert RSS item: {}", e);
                    continue;
                }
            };
        }

        Err(anyhow!(
            "Message queue subscriber is broken for subject ( {RSS_QUEUE_NAME} )"
        ))
    }
}
