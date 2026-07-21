use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub roles: Vec<String>,
    /// Id do tenant (UUID em string) dono dos dados — autoridade para o isolamento.
    pub tenant_id: String,
    /// Slug/subdomínio do tenant — usado apenas para casar com o host da requisição.
    #[serde(default)]
    pub tenant_slug: String,
    pub exp: u64,
    pub iat: u64,
}

#[allow(clippy::too_many_arguments)]
pub fn encode_token(
    usuario_id: Uuid,
    username: &str,
    roles: Vec<String>,
    tenant_id: &str,
    tenant_slug: &str,
    secret: &str,
    expiry_hours: u64,
) -> Result<String, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(AppError::infra)?
        .as_secs();

    let claims = Claims {
        sub: usuario_id.to_string(),
        username: username.to_owned(),
        roles,
        tenant_id: tenant_id.to_owned(),
        tenant_slug: tenant_slug.to_owned(),
        iat: now,
        exp: now + expiry_hours * 3600,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(AppError::infra)
}

pub fn decode_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackofficeClaims {
    pub sub: String,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub exp: u64,
    pub iat: u64,
}

pub fn encode_backoffice_token(
    user_id: Uuid,
    username: &str,
    role: &str,
    permissions: Vec<String>,
    secret: &str,
) -> Result<String, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(AppError::infra)?
        .as_secs();

    let claims = BackofficeClaims {
        sub: user_id.to_string(),
        username: username.to_owned(),
        role: role.to_owned(),
        permissions,
        iat: now,
        exp: now + 8 * 3600,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(AppError::infra)
}

pub fn decode_backoffice_token(
    token: &str,
    secret: &str,
) -> Result<BackofficeClaims, jsonwebtoken::errors::Error> {
    decode::<BackofficeClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims)
}
