//! `Idempotency-Key` replay for `POST /me/watch` and `POST /me/answers`.
//! Phones retry on bad networks; the first response is stored for 24 h and replayed.
//! The same key with a different payload is rejected (422) so a bug cannot silently
//! map two actions onto one.

use crate::cache::{keys, Cache};
use crate::crypto::sha256_hex;
use crate::error::ApiError;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

pub const TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredResponse {
    pub status: u16,
    pub body: serde_json::Value,
    pub fingerprint: String,
}

pub fn key_from_headers(headers: &HeaderMap) -> Result<Option<String>, ApiError> {
    match headers
        .get("idempotency-key")
        .map(|v| v.to_str().map(|s| s.trim().to_string()))
    {
        None => Ok(None),
        Some(Ok(s)) if s.is_empty() => Ok(None),
        Some(Ok(s)) if s.len() > 128 => Err(ApiError::BadRequest(
            "Idempotency-Key must be at most 128 characters".into(),
        )),
        Some(Ok(s)) => Ok(Some(s)),
        Some(Err(_)) => Err(ApiError::BadRequest("Idempotency-Key must be ASCII".into())),
    }
}

pub fn fingerprint(body: &[u8]) -> String {
    sha256_hex(body)
}

/// A stored replay, if any. Returns 422 when the key was used with another payload.
pub async fn replay(
    cache: &Cache,
    tenant: Uuid,
    key: &str,
    fp: &str,
) -> Result<Option<Response>, ApiError> {
    let Some(stored) = cache
        .get_json::<StoredResponse>(&keys::idempotency(tenant, key))
        .await
    else {
        return Ok(None);
    };
    if stored.fingerprint != fp {
        return Err(ApiError::UnprocessableEntity(
            "Idempotency-Key was already used with a different payload".into(),
        ));
    }
    let status = StatusCode::from_u16(stored.status).unwrap_or(StatusCode::OK);
    let mut response = (status, axum::Json(stored.body)).into_response();
    response.headers_mut().insert(
        "idempotent-replayed",
        header::HeaderValue::from_static("true"),
    );
    Ok(Some(response))
}

pub async fn store(
    cache: &Cache,
    tenant: Uuid,
    key: &str,
    fp: &str,
    status: StatusCode,
    body: &serde_json::Value,
) {
    let stored = StoredResponse {
        status: status.as_u16(),
        body: body.clone(),
        fingerprint: fp.to_string(),
    };
    cache
        .set_json(&keys::idempotency(tenant, key), &stored, TTL)
        .await;
}
