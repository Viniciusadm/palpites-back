use argon2::password_hash::{
    PasswordHash, PasswordHasher as ArgonPasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

use crate::application::auth::{PasswordHasher, TokenIssuer, TokenVerifier};
use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct Argon2PasswordHasher;

impl PasswordHasher for Argon2PasswordHasher {
    fn hash_password(&self, password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| AppError::Internal(format!("password hash failed: {error}")))
    }

    fn verify_password(&self, password: &str, password_hash: &str) -> Result<bool, AppError> {
        let parsed_hash = PasswordHash::new(password_hash)
            .map_err(|_| AppError::Unauthorized("invalid credentials".to_owned()))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

#[derive(Debug, Clone)]
pub struct JwtTokenService {
    secret: String,
}

impl JwtTokenService {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
        }
    }
}

impl TokenIssuer for JwtTokenService {
    fn issue_access_token(&self, user_id: &str) -> Result<String, AppError> {
        let expires_at = Utc::now() + Duration::days(30);
        let claims = AccessTokenClaims {
            sub: user_id.to_owned(),
            exp: expires_at.timestamp() as usize,
        };
        jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|error| AppError::Internal(format!("jwt issue failed: {error}")))
    }
}

impl TokenVerifier for JwtTokenService {
    fn verify_access_token(&self, token: &str) -> Result<String, AppError> {
        jsonwebtoken::decode::<AccessTokenClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims.sub)
        .map_err(|_| AppError::Unauthorized("invalid access token".to_owned()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessTokenClaims {
    sub: String,
    exp: usize,
}
