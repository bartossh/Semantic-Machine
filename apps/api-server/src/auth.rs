use crate::{config::JwtConfig, models::Claims};
use actix_web::http::header::USER_AGENT;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};

pub struct Authenticator {
    secret: String,
    expiration: Duration,
    issuer: String,
    audience: String,
}

impl Authenticator {
    /// Create a new instance of Authenticator.
    ///
    /// # Arguments
    /// * `config` - A reference to the JWT configuration.
    ///
    /// # Returns
    /// A new instance of Authenticator.
    pub fn new(config: &JwtConfig) -> Self {
        Authenticator {
            secret: config.secret.clone(),
            expiration: Duration::hours(config.expiration_hours),
            issuer: config.issuer.clone(),
            audience: config.audience.clone(),
        }
    }

    /// Generate a JWT token for the given email and name.
    ///
    /// # Arguments
    /// * `email` - The email address of the user.
    /// * `name` - The name of the user.
    ///
    /// # Returns
    /// A JWT token as a string.
    #[inline(always)]
    pub fn generate_jwt(
        &self,
        user_id: &str,
        solana_public_key: &str,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let expiration = Utc::now()
            .checked_add_signed(self.expiration)
            .expect("valid timestamp")
            .timestamp();

        let claims = Claims {
            sub: solana_public_key.to_string(),
            user_id: user_id.to_string(),
            name: format!("{user_id}-{solana_public_key}"),
            exp: expiration,
            iat: Utc::now().timestamp(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
    }

    /// Validate a JWT token and return the claims.
    ///
    /// # Arguments
    /// * `token` - The JWT token to validate.
    ///
    /// # Returns
    /// The claims if the token is valid.
    #[inline(always)]
    pub fn validate_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }
}
