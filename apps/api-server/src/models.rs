use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sqlx::Arguments;
use sqlx::Row;
use sqlx::postgres::PgArguments;
use sqlx::prelude::FromRow;
use std::collections::HashMap;
use utoipa::IntoParams;
use utoipa::ToSchema;
use validator::Validate;

use crate::{
    database::StoreReadBulkEntities, impl_read_bulk_by_ids, impl_read_bulk_multiple,
    impl_store_bulk,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, FromRow, Validate)]
pub struct SolanaUser {
    pub solana_wallet_public_key: [u8; 32],
    pub created_at: i64,
}

impl_store_bulk!(
    SolanaUser,
    [u8; 32],
    "users",
    [solana_wallet_public_key, created_at],
    "solana_wallet_public_key",
);

impl_read_bulk_multiple!(
    SolanaUser,
    "users",
    [user_id, solana_wallet_public_key, created_at],
    &HashMap<String, String>
);

impl_read_bulk_by_ids!(
    SolanaUser,
    [u8; 32],
    "users",
    [user_id, solana_wallet_public_key, created_at],
    "solana_wallet_public_key",
);

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub solana_wallet_public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
pub struct RegisterRequest {
    /// Solana wallet public key
    pub solana_wallet_public_key: String,
    /// Temporary token from Telegram
    pub token: String,
    /// Expiration time of the token
    pub expires_at: u64,
    /// Wallet signature to prove ownership
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
pub struct LoginRequest {
    /// Solana wallet public key
    pub solana_wallet_public_key: String,
    /// Temporary token from Telegram
    pub token: String,
    /// Expiration time of the token
    pub expires_at: u64,
    /// Wallet signature to prove ownership
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Claims {
    pub sub: String,
    pub user_id: String,
    pub name: String,
    pub exp: i64,
    pub iat: i64,
    pub aud: String,
    pub iss: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}
