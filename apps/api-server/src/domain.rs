#![allow(dead_code)]
use crate::{
    auth::Authenticator, database::PostgresStorageGateway, database::StoreInsertBulk,
    database::StoreReadBulkEntities, models::SolanaUser,
};
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::{convert::TryInto, time::SystemTime};
use thiserror::Error;
use tracing::info;
use validator::Validate;

const TOKEN_LIFETIME_MS: u64 = 5 * 60 * 1000;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum Error {
    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User not found")]
    UserNotFound,

    #[error("Parsing failure: {0}")]
    ParsingFailure(String),

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,
}

fn parse_pubkey(base58: &str) -> Result<[u8; 32], Error> {
    let decoded: Vec<u8> = bs58::decode(base58)
        .into_vec()
        .map_err(|e| Error::ParsingFailure(e.to_string()))?;

    let arr: [u8; 32] = decoded.try_into().map_err(|v: Vec<u8>| {
        Error::ParsingFailure(format!("expected 32 bytes, got {}", v.len()))
    })?;

    Ok(arr)
}

fn parse_signature(base58: &str) -> Result<[u8; 64], Error> {
    let decoded: Vec<u8> = bs58::decode(base58)
        .into_vec()
        .map_err(|e| Error::ParsingFailure(e.to_string()))?;

    let arr: [u8; 64] = decoded.try_into().map_err(|v: Vec<u8>| {
        Error::ParsingFailure(format!("expected 64 bytes, got {}", v.len()))
    })?;

    Ok(arr)
}

fn verify_signature(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> Result<()> {
    let public_key = VerifyingKey::from_bytes(public_key)?;
    let signature = Signature::from_bytes(signature);

    match public_key.verify(message, &signature) {
        Ok(()) => Ok(()),
        Err(e) => {
            info!("Invalid signature: {}", e);
            Err(Error::InvalidCredentials)?
        }
    }
}

/// Domain is contains business logic for the application.
pub struct Domain {
    storage: PostgresStorageGateway,
    auth: Authenticator,
    mac: Hmac<Sha256>,
    server_origin: String,
}

impl Domain {
    /// Creates a new instance of the Domain struct.
    ///
    /// # Arguments
    /// * `storage` - The storage gateway to use for data persistence.
    /// * `auth` - The authentication gateway to use for user authentication.
    /// * `generator_secret` - The generator secret to use for generating tokens.
    ///
    /// # Returns
    /// A new instance of the Domain struct.
    pub fn try_new(
        storage: PostgresStorageGateway,
        auth: Authenticator,
        generator_secret: [u8; 32],
        server_origin: String,
    ) -> Result<Self> {
        let mac = HmacSha256::new_from_slice(generator_secret.as_ref())
            .context("Wrong genrator secret key length")?;
        Ok(Self {
            storage,
            auth,
            mac,
            server_origin,
        })
    }

    pub async fn issue_token_challenge_base64(
        &self,
        solana_wallet: &str,
        offer_id: Option<u64>,
    ) -> Result<String> {
        let expires_at = Utc::now().timestamp_millis() as u64 + TOKEN_LIFETIME_MS;
        let solana_wallet_public_key = parse_pubkey(solana_wallet)?;
        let candidate_token =
            self.generate_token(&solana_wallet_public_key, expires_at, offer_id)?;
        Ok(general_purpose::URL_SAFE_NO_PAD.encode(candidate_token))
    }

    /// Register telegram user
    ///
    /// # Arguments
    /// * `token_b64` - The token to register in base64 format.
    /// * `expires_at` - The expiration time of the token.
    /// * `solana_wallet_public_key` - The solana wallet public key to register.
    /// * `signature` - The signature to verify.
    ///
    /// # Returns
    /// A result indicating success or failure.
    #[inline(always)]
    pub async fn register(
        &self,
        token_b64: &str,
        expires_at: u64,
        solana_wallet_public_key: &str,
        signature: &str,
    ) -> Result<()> {
        let solana_wallet_public_key = parse_pubkey(solana_wallet_public_key)?;
        let candidate_token = self.generate_token(&solana_wallet_public_key, expires_at, None)?;
        let token = general_purpose::URL_SAFE_NO_PAD.decode(token_b64)?;

        if candidate_token != token {
            return Err(Error::InvalidToken.into());
        }

        if expires_at
            < SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_millis() as u64
        {
            return Err(Error::TokenExpired.into());
        }

        let users_result: Vec<SolanaUser> = self
            .storage
            .read_bulk_by_ids(&[solana_wallet_public_key])
            .await?;
        if !users_result.is_empty() {
            return Err(Error::UserAlreadyExists.into());
        }
        let signature = parse_signature(signature)?;

        verify_signature(&solana_wallet_public_key, &token, &signature)?;

        let solana_user = SolanaUser {
            solana_wallet_public_key,
            created_at: Utc::now().timestamp_millis(),
        };
        solana_user.validate()?;

        self.storage.insert_bulk(&[solana_user]).await?;

        Ok(())
    }

    /// Verify the signature of a login request.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user.
    /// * `token_b64` - The token to verify.
    /// * `expires_at` - The expiration time of the token.
    /// * `signature` - The signature to verify.
    ///
    /// # Returns
    /// * `Result<String>` - JWT token or error message otherwise.
    #[inline(always)]
    pub async fn login(
        &self,
        solana_wallet: &str,
        token_b64: &str,
        expires_at: u64,
        signature: &str,
    ) -> Result<String> {
        let solana_wallet_public_key = parse_pubkey(solana_wallet)?;
        let candidate_token = self.generate_token(&solana_wallet_public_key, expires_at, None)?;
        let token = general_purpose::URL_SAFE_NO_PAD.decode(token_b64)?;

        if candidate_token != token {
            return Err(Error::InvalidToken.into());
        }

        if expires_at
            < SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_millis() as u64
        {
            return Err(Error::TokenExpired.into());
        }

        let solana_user: SolanaUser = self
            .storage
            .read_bulk_by_ids(&[solana_wallet_public_key])
            .await?
            .into_iter()
            .next()
            .ok_or(Error::UserNotFound)?;

        let signature = parse_signature(signature)?;

        verify_signature(&solana_user.solana_wallet_public_key, &token, &signature)?;

        let solana_wallet_public_key =
            bs58::encode(solana_user.solana_wallet_public_key).into_string();

        let jwt = self
            .auth
            .generate_jwt(solana_wallet, &solana_wallet_public_key)?;

        Ok(jwt)
    }

    fn generate_token(
        &self,
        solana_wallet: &[u8],
        expires_at: u64,
        offer_id: Option<u64>,
    ) -> Result<Vec<u8>> {
        let mut mac = self.mac.clone();
        let data = if let Some(offer_id) = offer_id {
            format!("{solana_wallet:x?}:{offer_id}:{expires_at}")
        } else {
            format!("{solana_wallet:x?}:{expires_at:x}")
        };
        mac.update(data.as_bytes());
        Ok(mac.finalize().into_bytes().to_vec())
    }
}
