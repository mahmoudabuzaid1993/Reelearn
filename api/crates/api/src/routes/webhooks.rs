//! Inbound webhooks: WhatsApp (messages + receipts) and generation providers. Signature-verified.

use crate::error::ApiError;
use crate::routes::{Path, Query};
use crate::services::{learn, whatsapp_bot};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use hmac::{Hmac, Mac};
use reelearn_db::repos::{deliveries, jobs, users};
use reelearn_domain::phone;
use serde::Deserialize;
use sha2::Sha256;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/webhooks/whatsapp",
            get(whatsapp_verify).post(whatsapp_inbound),
        )
        .route("/webhooks/providers/{provider}", post(provider_callback))
}

fn verify_hmac(secret: &str, body: &[u8], header: Option<&str>) -> bool {
    let Some(sig) = header else { return false };
    let sig = sig.trim().trim_start_matches("sha256=");
    let Ok(expected) = hex::decode(sig) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&expected).is_ok()
}

#[derive(Deserialize)]
struct HubQuery {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
}

/// Meta's subscription handshake.
async fn whatsapp_verify(
    State(state): State<AppState>,
    Query(q): Query<HubQuery>,
) -> Result<Response, ApiError> {
    let expected = state.config.whatsapp_verify_token.as_deref();
    if q.mode.as_deref() == Some("subscribe")
        && expected.is_some()
        && q.verify_token.as_deref() == expected
    {
        return Ok((StatusCode::OK, q.challenge.unwrap_or_default()).into_response());
    }
    Err(ApiError::Forbidden("verify token mismatch".into()))
}

#[derive(Deserialize, Default)]
struct WaEnvelope {
    #[serde(default)]
    entry: Vec<WaEntry>,
}
#[derive(Deserialize, Default)]
struct WaEntry {
    #[serde(default)]
    changes: Vec<WaChange>,
}
#[derive(Deserialize, Default)]
struct WaChange {
    #[serde(default)]
    value: WaValue,
}
#[derive(Deserialize, Default)]
struct WaValue {
    #[serde(default)]
    messages: Vec<WaMessage>,
    #[serde(default)]
    statuses: Vec<WaStatus>,
}
#[derive(Deserialize)]
struct WaMessage {
    id: String,
    from: String,
    #[serde(default)]
    text: Option<WaText>,
    #[serde(default)]
    button: Option<WaButton>,
    #[serde(default)]
    interactive: Option<WaInteractive>,
}
#[derive(Deserialize)]
struct WaText {
    body: String,
}
#[derive(Deserialize)]
struct WaButton {
    text: String,
}
#[derive(Deserialize)]
struct WaInteractive {
    #[serde(default)]
    button_reply: Option<WaReply>,
    #[serde(default)]
    list_reply: Option<WaReply>,
}
#[derive(Deserialize)]
struct WaReply {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
}
#[derive(Deserialize)]
struct WaStatus {
    id: String,
    status: String,
}

impl WaMessage {
    fn text(&self) -> Option<String> {
        if let Some(t) = &self.text {
            return Some(t.body.clone());
        }
        if let Some(b) = &self.button {
            return Some(b.text.clone());
        }
        if let Some(i) = &self.interactive {
            if let Some(r) = i.button_reply.as_ref().or(i.list_reply.as_ref()) {
                return Some(if r.id.is_empty() {
                    r.title.clone()
                } else {
                    r.id.clone()
                });
            }
        }
        None
    }
}

async fn whatsapp_inbound(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    if let Some(secret) = state.config.whatsapp_app_secret.as_deref() {
        let sig = headers
            .get("x-hub-signature-256")
            .and_then(|v| v.to_str().ok());
        if !verify_hmac(secret, &body, sig) {
            return Err(ApiError::Unauthorized("invalid webhook signature".into()));
        }
    } else if !state.is_development() {
        return Err(ApiError::Unauthorized(
            "WHATSAPP_APP_SECRET is not configured".into(),
        ));
    }
    let envelope: WaEnvelope = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(format!("invalid webhook payload: {e}")))?;
    for entry in envelope.entry {
        for change in entry.changes {
            for status in change.value.statuses {
                handle_status(&state, &status).await;
            }
            for message in change.value.messages {
                if let Err(e) = handle_message(&state, &message).await {
                    tracing::warn!(error = %e, from = %message.from, "whatsapp message not handled");
                }
            }
        }
    }
    Ok(StatusCode::OK)
}

async fn handle_status(state: &AppState, status: &WaStatus) {
    let mapped = match status.status.as_str() {
        "delivered" => reelearn_domain::DeliveryStatus::Delivered,
        "read" => reelearn_domain::DeliveryStatus::Read,
        "failed" => reelearn_domain::DeliveryStatus::Failed,
        _ => return,
    };
    if let Ok(mut tx) = state.db.admin_tx().await {
        let _ = deliveries::mark_by_provider_ref(&mut tx, &status.id, mapped).await;
        let _ = tx.commit().await;
    }
}

async fn handle_message(state: &AppState, message: &WaMessage) -> Result<(), ApiError> {
    let Some(text) = message.text() else {
        return Ok(());
    };
    let phone_e164 =
        phone::normalize(&message.from).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let candidates = {
        let mut tx = state.db.admin_tx().await?;
        users::find_by_phone_admin(&mut tx, &phone_e164).await?
    };
    let Some(user) = candidates.first() else {
        tracing::info!(phone = %phone_e164, "whatsapp message from an unknown number");
        return Ok(());
    };
    if candidates.len() > 1 {
        tracing::warn!(phone = %phone_e164, "phone belongs to several workspaces; using the oldest");
    }
    let mut tx = state.db.tenant_tx(user.tenant_id).await?;
    if deliveries::whatsapp_inbound_seen(tx.conn(), &message.id).await? {
        return Ok(()); // Meta retries: idempotent
    }
    deliveries::log_whatsapp(
        tx.conn(),
        user.tenant_id,
        Some(user.id),
        &phone_e164,
        "in",
        Some(&text),
        None,
        Some(&message.id),
    )
    .await?;
    let ctx = learn::load_ctx(&mut tx, user.id).await?;
    let replies = whatsapp_bot::handle(state, &mut tx, &ctx, &text).await?;
    whatsapp_bot::queue_replies(&mut tx, &ctx, &phone_e164, &replies).await?;
    tx.commit().await?;
    Ok(())
}

/// Generation provider callbacks: verified, then handed to the worker as a `provider_callback` job.
async fn provider_callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    if !matches!(provider.as_str(), "higgsfield" | "veo" | "voice" | "asr") {
        return Err(ApiError::NotFound(format!("unknown provider {provider}")));
    }
    if let Some(secret) = state.config.provider_webhook_secret.as_deref() {
        let sig = headers
            .get("x-signature")
            .or_else(|| headers.get("x-hub-signature-256"))
            .and_then(|v| v.to_str().ok());
        if !verify_hmac(secret, &body, sig) {
            return Err(ApiError::Unauthorized("invalid webhook signature".into()));
        }
    } else if !state.is_development() {
        return Err(ApiError::Unauthorized(
            "PROVIDER_WEBHOOK_SECRET is not configured".into(),
        ));
    }
    let payload: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(format!("invalid payload: {e}")))?;
    let mut tx = state.db.admin_tx().await?;
    jobs::enqueue(
        &mut tx,
        None,
        "provider_callback",
        serde_json::json!({ "provider": provider, "payload": payload }),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_signature_round_trip() {
        let body = b"{\"a\":1}";
        let mut mac = Hmac::<Sha256>::new_from_slice(b"secret").expect("mac");
        mac.update(body);
        let sig = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        assert!(verify_hmac("secret", body, Some(&sig)));
        assert!(!verify_hmac("other", body, Some(&sig)));
        assert!(!verify_hmac("secret", body, None));
    }
}
