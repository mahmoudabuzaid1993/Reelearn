//! `/platform/*`: the people who run Reelearn (not a company role). They set the platform's own
//! provider keys and prices, each company's AI plan (may it use the platform's keys, monthly
//! budget, markup) and export usage for invoicing. Gated by `platform_operators`, checked on
//! every request so revoking takes effect at once.

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::routes::settings::{month_window, probe};
use crate::routes::{record_audit, Json, Path, Query};
use crate::services::ai::{self, has_credentials, kind_static, PROVIDER_KINDS};
use crate::state::AppState;
use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::Router;
use reelearn_db::repos::ai as repo;
use reelearn_domain::ai::AiPlan;
use reelearn_domain::IntegrationKind;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::str::FromStr;
use uuid::Uuid;

pub fn routes(state: AppState) -> Router<AppState> {
    let operator = Router::new()
        .route("/platform/integrations", get(list))
        .route("/platform/integrations/{kind}", put(upsert))
        .route("/platform/integrations/{kind}/test", post(test))
        .route("/platform/tenants", get(tenants))
        .route("/platform/tenants/{tenant_id}/ai-plan", put(set_plan))
        .route("/platform/usage.csv", get(usage_csv))
        .route_layer(middleware::from_fn_with_state(state, require_operator));
    Router::new().route("/platform/me", get(me)).merge(operator)
}

async fn is_operator(state: &AppState, user_id: Uuid) -> Result<bool, ApiError> {
    let mut tx = state.db.admin_tx().await?;
    let ok = repo::is_platform_operator_admin(&mut tx, user_id).await?;
    tx.commit().await?;
    Ok(ok)
}

async fn require_operator(
    State(state): State<AppState>,
    user: AuthUser,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if !is_operator(&state, user.user_id).await? {
        return Err(ApiError::Forbidden(
            "this is for the people who run the platform".into(),
        ));
    }
    Ok(next.run(req).await)
}

#[derive(Serialize)]
struct PlatformMe {
    operator: bool,
}

async fn me(State(state): State<AppState>, auth: AuthUser) -> Result<Json<PlatformMe>, ApiError> {
    Ok(Json(PlatformMe {
        operator: is_operator(&state, auth.user_id).await?,
    }))
}

// ---------------------------------------------------------------- platform keys

#[derive(Serialize)]
struct PlatformIntegration {
    kind: String,
    /// `connected` when the platform has usable credentials (from the console or the server environment).
    status: String,
    /// Where the credentials come from: `console`, `environment`, or null.
    source: Option<&'static str>,
    last_error: Option<String>,
    /// Secrets masked; options and prices shown.
    public_config: Value,
}

fn mask(kind: &str, cfg: &Map<String, Value>) -> Value {
    let secrets = IntegrationKind::from_str(kind)
        .map(|k| k.secret_keys())
        .unwrap_or(&[]);
    Value::Object(
        cfg.iter()
            .map(|(k, v)| {
                if secrets.contains(&k.as_str()) {
                    (k.clone(), Value::String("••••".into()))
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect(),
    )
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<PlatformIntegration>>, ApiError> {
    let mut tx = state.db.admin_tx().await?;
    let rows = repo::platform_integrations_admin(&mut tx).await?;
    tx.commit().await?;
    let merged = ai::platform_configs(&state).await?;
    Ok(Json(
        PROVIDER_KINDS
            .iter()
            .map(|kind| {
                let cfg = merged.get(kind).cloned().unwrap_or_default();
                let row = rows.iter().find(|r| r.kind == *kind);
                let console = row.is_some_and(|r| r.config_enc.is_some());
                let env = has_credentials(kind, &ai::env_config(kind));
                let usable = has_credentials(kind, &cfg);
                PlatformIntegration {
                    kind: (*kind).to_string(),
                    status: match row.map(|r| r.status.as_str()) {
                        Some("error") => "error".into(),
                        _ if usable => "connected".into(),
                        _ => "disconnected".into(),
                    },
                    source: match (usable, console, env) {
                        (false, _, _) => None,
                        (true, true, _) => Some("console"),
                        (true, false, _) => Some("environment"),
                    },
                    last_error: row.and_then(|r| r.last_error.clone()),
                    public_config: mask(kind, &cfg),
                }
            })
            .collect(),
    ))
}

fn parse_kind(kind: &str) -> Result<(&'static str, IntegrationKind), ApiError> {
    let k =
        kind_static(kind).ok_or_else(|| ApiError::NotFound(format!("unknown provider {kind}")))?;
    let ik = IntegrationKind::from_str(k)
        .map_err(|_| ApiError::NotFound(format!("unknown provider {kind}")))?;
    Ok((k, ik))
}

async fn upsert(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(kind): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Vec<PlatformIntegration>>, ApiError> {
    let (kind, ik) = parse_kind(&kind)?;
    let incoming = body
        .as_object()
        .ok_or_else(|| ApiError::BadRequest("body must be a JSON object".into()))?;
    if let Some(bad) = incoming.keys().find(|k| !ai::allowed_key(ik, k)) {
        return Err(ApiError::field(bad, "is not a setting of this provider"));
    }
    let mut tx = state.db.admin_tx().await?;
    let existing = repo::platform_integrations_admin(&mut tx)
        .await?
        .into_iter()
        .find(|r| r.kind == kind)
        .and_then(|r| r.config_enc)
        .and_then(|b| state.secrets.decrypt_json(&b).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    let mut merged = existing;
    for (k, v) in incoming {
        if v.is_null() {
            merged.remove(k);
        } else if v.as_str() == Some("••••") {
            continue;
        } else {
            merged.insert(k.clone(), v.clone());
        }
    }
    let status = if has_credentials(kind, &merged) {
        "connected"
    } else {
        "disconnected"
    };
    let blob = state
        .secrets
        .encrypt_json(&Value::Object(merged))
        .map_err(ApiError::internal)?;
    repo::upsert_platform_integration_admin(&mut tx, kind, status, &blob, auth.user_id).await?;
    tx.commit().await?;
    tracing::info!(operator = %auth.user_id, kind, keys = ?incoming.keys().collect::<Vec<_>>(), "platform integration updated");
    list(State(state)).await
}

#[derive(Serialize)]
struct TestResult {
    ok: bool,
    message: String,
}

async fn test(
    State(state): State<AppState>,
    Path(kind): Path<String>,
) -> Result<Json<TestResult>, ApiError> {
    let (kind, ik) = parse_kind(&kind)?;
    let cfg = ai::platform_configs(&state)
        .await?
        .remove(kind)
        .unwrap_or_default();
    let result = if !has_credentials(kind, &cfg) {
        TestResult {
            ok: false,
            message: "credentials missing".into(),
        }
    } else {
        match probe(&state, ik, &Value::Object(cfg)).await {
            None => TestResult {
                ok: true,
                message: "credentials present and accepted".into(),
            },
            Some(m) => TestResult {
                ok: false,
                message: m,
            },
        }
    };
    let mut tx = state.db.admin_tx().await?;
    repo::set_platform_status_admin(
        &mut tx,
        kind,
        if result.ok { "connected" } else { "error" },
        (!result.ok).then_some(result.message.as_str()),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(result))
}

// ---------------------------------------------------------------- companies and plans

#[derive(Deserialize)]
struct MonthQuery {
    month: Option<String>,
}

#[derive(Serialize)]
struct TenantAi {
    id: Uuid,
    slug: String,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    plan: AiPlan,
    calls: i64,
    company_calls: i64,
    platform_calls: i64,
    /// What the platform paid its vendors for this company (priced calls only).
    platform_cost_usd: f64,
    /// `platform_cost_usd` with the plan's markup: the amount to invoice.
    billable_usd: f64,
    /// The company's own-key spend, as priced.
    company_cost_usd: f64,
    unpriced_calls: i64,
    /// Share of the monthly budget used, 0..1, when there is a budget.
    budget_used: Option<f64>,
}

#[derive(Serialize)]
struct TenantsAi {
    from: chrono::DateTime<chrono::Utc>,
    to: chrono::DateTime<chrono::Utc>,
    tenants: Vec<TenantAi>,
}

async fn tenants(
    State(state): State<AppState>,
    Query(q): Query<MonthQuery>,
) -> Result<Json<TenantsAi>, ApiError> {
    let (from, to) = month_window(q.month.as_deref())?;
    let mut tx = state.db.admin_tx().await?;
    let rows = repo::tenants_with_usage_admin(&mut tx, from, to).await?;
    tx.commit().await?;
    let tenants = rows
        .into_iter()
        .map(|r| {
            let plan: AiPlan = serde_json::from_value(r.ai_plan).unwrap_or_default();
            let billable = plan.billable(r.platform_cost);
            TenantAi {
                budget_used: plan.monthly_budget_usd.map(|b| {
                    if b > 0.0 {
                        (billable / b).min(1.0)
                    } else {
                        1.0
                    }
                }),
                id: r.id,
                slug: r.slug,
                name: r.name,
                created_at: r.created_at,
                calls: r.calls,
                company_calls: r.company_calls,
                platform_calls: r.platform_calls,
                platform_cost_usd: r.platform_cost,
                billable_usd: billable,
                company_cost_usd: r.company_cost,
                unpriced_calls: r.unpriced_calls,
                plan,
            }
        })
        .collect();
    Ok(Json(TenantsAi { from, to, tenants }))
}

async fn set_plan(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(tenant_id): Path<Uuid>,
    Json(plan): Json<AiPlan>,
) -> Result<Json<AiPlan>, ApiError> {
    plan.validate().map_err(|m| ApiError::field("plan", &m))?;
    if let Some(bad) = plan.allowed.iter().find(|k| kind_static(k).is_none()) {
        return Err(ApiError::field(
            "allowed",
            &format!("unknown provider {bad}"),
        ));
    }
    let mut admin = state.db.admin_tx().await?;
    let before = repo::plan(&mut admin, tenant_id).await?;
    repo::set_plan_admin(&mut admin, tenant_id, &plan).await?;
    admin.commit().await?;
    // The company's own audit log shows that its AI plan changed, and to what.
    let mut tx = state.db.tenant_tx(tenant_id).await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "ai.plan.update",
        "tenant",
        Some(tenant_id),
        Some(serde_json::json!({ "before": before, "after": plan })),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(plan))
}

/// Invoice lines for a month, one per company × provider × capability × key source × model.
async fn usage_csv(
    State(state): State<AppState>,
    Query(q): Query<MonthQuery>,
) -> Result<Response, ApiError> {
    let (from, to) = month_window(q.month.as_deref())?;
    let mut tx = state.db.admin_tx().await?;
    let lines = repo::usage_lines_admin(&mut tx, from, to).await?;
    let plans: std::collections::HashMap<String, AiPlan> =
        repo::tenants_with_usage_admin(&mut tx, from, to)
            .await?
            .into_iter()
            .map(|t| {
                (
                    t.slug,
                    serde_json::from_value(t.ai_plan).unwrap_or_default(),
                )
            })
            .collect();
    tx.commit().await?;
    let mut w = csv::Writer::from_writer(Vec::new());
    let header_row = [
        "company",
        "provider",
        "capability",
        "key_source",
        "model",
        "calls",
        "input_tokens",
        "output_tokens",
        "units",
        "cost_usd",
        "markup_pct",
        "billable_usd",
        "unpriced_calls",
    ];
    w.write_record(header_row).map_err(ApiError::internal)?;
    for l in &lines {
        let plan = plans.get(&l.tenant_slug).cloned().unwrap_or_default();
        let platform = l.key_source == "platform";
        let billable = match (platform, l.cost_usd) {
            (true, Some(c)) => format!("{:.6}", plan.billable(c)),
            (false, _) => "0".into(),
            (true, None) => String::new(),
        };
        w.write_record([
            l.tenant_slug.clone(),
            l.provider.clone(),
            l.capability.clone(),
            l.key_source.clone(),
            l.model.clone().unwrap_or_default(),
            l.calls.to_string(),
            l.input_tokens.to_string(),
            l.output_tokens.to_string(),
            l.units.to_string(),
            l.cost_usd.map(|c| format!("{c:.6}")).unwrap_or_default(),
            if platform {
                format!("{}", plan.markup_pct)
            } else {
                String::new()
            },
            billable,
            l.unpriced_calls.to_string(),
        ])
        .map_err(ApiError::internal)?;
    }
    let bytes = w.into_inner().map_err(ApiError::internal)?;
    let name = format!("reelearn-ai-usage-{}.csv", from.format("%Y-%m"));
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}
