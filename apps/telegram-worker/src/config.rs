use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub telegram_api_id: i32,
    pub telegram_api_hash: String,
}

impl TelegramConfig {
    pub fn try_from_env() -> Result<Self> {
        let telegram_api_id = env::var("TELEGRAM_API_ID")
            .context("TELEGRAM_API_ID must be set")?
            .parse()?;
        let telegram_api_hash =
            env::var("TELEGRAM_API_HASH").context("TELEGRAM_API_HASH must be set")?;
        Ok(Self {
            telegram_api_id,
            telegram_api_hash,
        })
    }
}
