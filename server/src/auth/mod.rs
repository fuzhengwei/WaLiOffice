pub mod middleware;

use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::models::User;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id
    pub username: String,
    pub role: String,
    pub exp: usize,
}

pub fn create_token(user: &User) -> Result<String> {
    let cfg = crate::config::config();
    let exp = (Utc::now() + Duration::hours(cfg.jwt_expiry_hours)).timestamp() as usize;
    let claims = Claims {
        sub: user.id.clone(),
        username: user.username.clone(),
        role: user.role.clone(),
        exp,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(cfg.jwt_secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify_token(token: &str) -> Result<Claims> {
    let cfg = crate::config::config();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(cfg.jwt_secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}
