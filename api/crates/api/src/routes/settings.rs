//! `/admin/integrations` (secrets write-only, AES-GCM at rest) and `/admin/audit`.

use crate::auth::AuthUser;
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::{page_params, record_audit, Json, Path, Query};
use crate::state::AppState;
use axum::extract::State;
use axum::routing::{get, put};
use axum::Router;
use reelearn_db::repos::{audit, integrations};
use reelearn_domain::cursor::{page, Cursor};
use reelearn_domain::IntegrationKind;
use serde::Deserialize;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/integrations", get(list))
        .route("/admin/integrations/{kind}", put(upsert))
        .route("/admin/integrations/{kind}/test", axum::routing::post(test))
        .route("/admin/audit", get(audit_list))
}

fn public_config(kind: IntegrationKind, config: &serde_json::Value) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    if let Some(obj) = config.as_object() {
        for (k, v) in obj {
            if kind.secret_keys().contains(&k.as_str()) {
                out.insert(k.clone(), serde_json::Value::String("••••".into()));
            } else {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    serde_json::Value::Object(out)
}

/// Decrypt the stored config.
pub fn stored_config(state: &AppState, row: &integrations::IntegrationRow) -> serde_json::Value {
    row.config_enc
        .as_deref()
        .and_then(|b| state.secrets.decrypt_json(b).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn view(state: &AppState, row: &integrations::IntegrationRow) -> Integration {
    let config = stored_config(state, row);
    Integration {
        kind: row.kind,
        status: row.status.clone(),
        last_sync_at: row.last_sync_at,
        last_error: row.last_error.clone(),
        public_config: public_config(row.kind, &config),
    }
}

async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Integration>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = integrations::list(tx.conn()).await?;
    tx.commit().await?;
    let out = IntegrationKind::ALL
        .iter()
        .map(|k| match rows.iter().find(|r| r.kind == *k) {
            Some(r) => view(&state, r),
            None => Integration {
                kind: *k,
                status: "disconnected".into(),
                last_sync_at: None,
                last_error: None,
                public_config: serde_json::json!({}),
            },
        })
        .collect();
    Ok(Json(out))
}

fn parse_kind(kind: &str) -> Result<IntegrationKind, ApiError> {
    kind.parse()
        .map_err(|_| ApiError::NotFound(format!("unknown integration kind {kind}")))
}

async fn upsert(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(kind): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<Integration>, ApiError> {
    let kind = parse_kind(&kind)?;
    let incoming = body
        .as_object()
        .ok_or_else(|| ApiError::BadRequest("body must be a JSON object".into()))?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    // Merge over the existing config so a PUT without secrets keeps the stored ones.
    let mut merged = match integrations::find(tx.conn(), kind).await? {
        Some(existing) => stored_config(&state, &existing)
            .as_object()
            .cloned()
            .unwrap_or_default(),
        None => serde_json::Map::new(),
    };
    for (k, v) in incoming {
        if v.is_null() {
            merged.remove(k);
        } else if v.as_str() == Some("••••") {
            continue; // masked value echoed back by a client: keep the stored secret
        } else {
            merged.insert(k.clone(), v.clone());
        }
    }
    let has_required = kind.required_keys().iter().all(|k| {
        merged
            .get(*k)
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    });
    let status = if has_required {
        "connected"
    } else {
        "disconnected"
    };
    let blob = state
        .secrets
        .encrypt_json(&serde_json::Value::Object(merged))
        .map_err(ApiError::internal)?;
    let row = integrations::upsert(tx.conn(), auth.tenant_id, kind, status, &blob).await?;
    let keys: Vec<&String> = incoming.keys().collect();
    record_audit(
        &mut tx,
        auth.user_id,
        "integration.update",
        "integration",
        Some(row.id),
        Some(serde_json::json!({ "kind": kind, "keys": keys })),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(view(&state, &row)))
}

#[derive(serde::Serialize)]
struct TestResult {
    ok: bool,
    message: String,
}

async fn test(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(kind): Path<String>,
) -> Result<Json<TestResult>, ApiError> {
    let kind = parse_kind(&kind)?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let row = integrations::find(tx.conn(), kind).await?;
    let result = match row {
        None => TestResult {
            ok: false,
            message: "not configured".into(),
        },
        Some(r) => {
            let config = stored_config(&state, &r);
            let missing: Vec<&str> = kind
                .required_keys()
                .iter()
                .copied()
                .filter(|k| {
                    config
                        .get(*k)
                        .and_then(|v| v.as_str())
                        .map(|s| s.is_empty())
                        .unwrap_or(true)
                })
                .collect();
            if !missing.is_empty() {
                integrations::set_status(
                    tx.conn(),
                    kind,
                    "error",
                    Some(&format!("missing {}", missing.join(", "))),
                )
                .await?;
                TestResult {
                    ok: false,
                    message: format!("missing: {}", missing.join(", ")),
                }
            } else {
                let message = probe(&state, kind, &config).await;
                let ok = message.is_none();
                integrations::set_status(
                    tx.conn(),
                    kind,
                    if ok { "connected" } else { "error" },
                    message.as_deref(),
                )
                .await?;
                TestResult {
                    ok,
                    message: message.unwrap_or_else(|| "credentials present and accepted".into()),
                }
            }
        }
    };
    tx.commit().await?;
    Ok(Json(result))
}

/// Live probe for providers with a cheap authenticated endpoint; `None` means OK.
async fn probe(
    state: &AppState,
    kind: IntegrationKind,
    config: &serde_json::Value,
) -> Option<String> {
    if state.is_development() {
        return None;
    }
    let get = |k: &str| {
        config
            .get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let req = match kind {
        IntegrationKind::AiClaude => state
            .http
            .get("https://api.anthropic.com/v1/models")
            .header("x-api-key", get("api_key"))
            .header("anthropic-version", "2023-06-01"),
        IntegrationKind::AiMuse => state
            .http
            .get("https://api.meta.ai/v1/models")
            .bearer_auth(get("api_key")),
        IntegrationKind::Whatsapp => state
            .http
            .get(format!(
                "https://graph.facebook.com/v20.0/{}",
                get("phone_number_id")
            ))
            .bearer_auth(get("access_token")),
        _ => return None,
    };
    match req.send().await {
        Ok(r) if r.status().is_success() => None,
        Ok(r) => Some(format!("provider answered {}", r.status())),
        Err(e) => Some(format!("provider unreachable: {e}")),
    }
}

#[derive(Deserialize)]
struct AuditQuery {
    entity: Option<String>,
    entity_id: Option<Uuid>,
    #[serde(flatten)]
    page: PageQuery,
}

async fn audit_list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Page<AuditEntry>>, ApiError> {
    let (cursor, limit) = page_params(&q.page)?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = audit::list(tx.conn(), q.entity.as_deref(), q.entity_id, cursor, limit).await?;
    tx.commit().await?;
    let (rows, next) = page(rows, limit, |r| Cursor { ts: r.at, id: r.id });
    let items = rows
        .into_iter()
        .map(|r| AuditEntry {
            id: r.id,
            actor: r.actor_id.map(|id| AuditActor {
                id,
                full_name: r.actor_name.clone().unwrap_or_default(),
            }),
            action: r.action,
            entity: r.entity,
            entity_id: r.entity_id,
            diff: r.diff,
            at: r.at,
        })
        .collect();
    Ok(Json(Page {
        items,
        next_cursor: next,
    }))
}
