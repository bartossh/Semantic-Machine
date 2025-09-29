use anyhow::{Context, Result};
use redis::AsyncCommands;
use std::env;

pub struct Config {
    pub redis_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let redis_host = env::var("REDIS_HOST").unwrap_or_default();
        let redis_port = env::var("REDIS_PORT")
            .unwrap_or("6379".to_string())
            .parse::<u16>()
            .context("REDIS_PORT must be a valid number")?;
        let redis_password = env::var("REDIS_PASSWORD").unwrap_or_default();
        let redis_database = env::var("REDIS_DATABASE")
            .unwrap_or("0".to_string())
            .parse::<u8>()
            .context("REDIS_DATABASE must be a valid number")?;
        let redis_url = env::var("REDIS_URL").unwrap_or(format!(
            "redis://:{redis_password}@{redis_host}:{redis_port}/{redis_database}"
        ));
        Ok(Self { redis_url })
    }
}

pub struct RedisMiddleware {
    client: redis::Client,
}

impl RedisMiddleware {
    pub fn new(url: &str) -> Result<Self> {
        let client = redis::Client::open(url)?;
        Ok(Self { client })
    }

    pub async fn store(&self, key: &str, value: &str) -> Result<()> {
        Ok(self
            .client
            .get_multiplexed_async_connection()
            .await?
            .set(key, value)
            .await?)
    }

    pub async fn retrieve(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .client
            .get_multiplexed_async_connection()
            .await?
            .get(key)
            .await?)
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        Ok(self
            .client
            .get_multiplexed_async_connection()
            .await?
            .del(key)
            .await?)
    }
}

#[cfg(feature = "integrations")]
#[cfg(test)]
mod test {
    use super::*;

    const REDIS_URL: &str = "redis://:password@localhost:6379";

    #[tokio::test]
    async fn test_store_and_retrieve() -> Result<()> {
        let middleware = RedisMiddleware::new(REDIS_URL)?;
        let key = "test_key_1";
        let value = "test_value_1";

        middleware.store(key, value).await?;
        let result = middleware.retrieve(key).await?;
        assert_eq!(result, Some(value.to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_delete() -> Result<()> {
        let middleware = RedisMiddleware::new(REDIS_URL)?;
        let key = "test_key_2";
        let value = "test_value_2";

        middleware.store(key, value).await?;
        middleware.delete(key).await?;
        let result = middleware.retrieve(key).await?;
        assert_eq!(result, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_retrieve() -> Result<()> {
        let middleware = RedisMiddleware::new(REDIS_URL)?;
        let key = "test_key_3";
        let value = "test_value_3";

        middleware.store(key, value).await?;
        let result = middleware.retrieve(key).await?;
        assert_eq!(result, Some(value.to_string()));

        Ok(())
    }
}
