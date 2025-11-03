use crate::data_model::AppState;
use axum::{
    extract::FromRequestParts, 
    http::{self, StatusCode, request::Parts}, 
    response::IntoResponse
};
use reqwest;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation, jwk::JwkSet};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

static AUTH0_DOMAIN: &str = "dev-rpx7rpje8hqht13l.us.auth0.com";
static AUTH0_ISSUER: &str = "https://dev-rpx7rpje8hqht13l.us.auth0.com/";
static AUTH0_AUDIENCE: &str = "http://localhost:3000/";


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub aud: serde_json::Value,
    pub exp: usize,
}

pub struct AuthUser {
  pub claims: Claims,
}

pub async fn fetch_jwks() -> Result<JwkSet, Box<dyn std::error::Error>> {
    let url = format!("https://{}/.well-known/jwks.json", AUTH0_DOMAIN);
    let res = reqwest::get(url).await?;
    let jwks = res.json::<JwkSet>().await?;
    Ok(jwks)
}

impl FromRequestParts<Arc<AppState>> for AuthUser {

    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, Self::Rejection> {
        // parse header for token and key id
        let auth_header = parts.headers.get(http::header::AUTHORIZATION).ok_or(AuthError::MissingToken)?;
        let auth_str = auth_header.to_str().map_err(|_| AuthError::InvalidToken)?;
        let token = auth_str.strip_prefix("Bearer ").ok_or(AuthError::InvalidToken)?;
        
        let header = decode_header(token).map_err(|_| AuthError::InvalidToken)?;
        let kid = header.kid.ok_or(AuthError::InvalidToken)?;

        // search for jwk with key id in AppState to get decoding key
        let jwk = state.jwks.find(&kid).ok_or(AuthError::InvalidToken)?;
        let decoding_key = DecodingKey::from_jwk(jwk).map_err(|_| AuthError::InvalidToken)?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[AUTH0_AUDIENCE]);
        validation.set_issuer(&[AUTH0_ISSUER]);

        // decode token into Claims struct with above values
        let token_data = decode::<Claims>(token, &decoding_key, &validation).map_err(|_| AuthError::InvalidToken)?;

        Ok(AuthUser { claims: token_data.claims })
    }
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
            let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authorization token"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
        };
        (status, message).into_response()
    }
}