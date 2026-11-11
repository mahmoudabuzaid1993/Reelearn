//! Which key each AI provider uses for a company, and the bill for it.
//!
//! For every provider kind: the company's own key (`integrations`, set in Settings), else the
//! platform's (`platform_integrations` set by the platform operator, falling back to the server
//! environment) when the company's AI plan allows it and this month's budget has room. The rules
//! are `reelearn_domain::ai::resolve_source`; this module loads and decrypts what they need.

use crate::error::ApiError;
use crate::state::AppState;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use reelearn_db::repos::{ai as repo, integrations, tenants};
use reelearn_db::settings::TenantSettings;
use reelearn_domain::ai::{
    resolve_source, AiPlan, AiRouting, KeySource, Prices, Unavailable, DEFAULT_CLAUDE_MODEL,
    DEFAULT_JEV_MODEL, DEFAULT_MUSE_MODEL,
};
use reelearn_domain::IntegrationKind;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

/// Provider kinds the platform layer covers, in display order.
pub const PROVIDER_KINDS: [&str; 7] = [
    "ai_claude",
    "ai_muse",
    "ai_typesafe",
    "ai_higgsfield",
    "voice",
    "asr",
    "whatsapp",
];

/// Non-secret config keys a company or the platform may set besides the credentials.
pub const OPTION_KEYS: [&str; 8] = [
    "model",
    "default_voice",
    "base_url",
    "provider",
    "phone_number_id",
    "price_input_per_mtok",
    "price_output_per_mtok",
    "price_per_call",
];

pub fn kind_static(kind: &str) -> Option<&'static str> {
    PROVIDER_KINDS.iter().copied().find(|k| *k == kind)
}

fn default_model(kind: &str) -> Option<&'static str> {
    match kind {
        "ai_claude" => Some(DEFAULT_CLAUDE_MODEL),
        "ai_muse" => Some(DEFAULT_MUSE_MODEL),
        "ai_typesafe" => Some(DEFAULT_JEV_MODEL),
        _ => None,
    }
}

fn default_prices(kind: &str) -> Prices {
    match kind {
        "ai_typesafe" => Prices::jev(),
        _ => Prices::default(),
    }
}

/// The server environment's keys for a kind, as a config object (the platform's last fallback).
pub fn env_config(kind: &str) -> Map<String, Value> {
    let env = |k: &str| {
        std::env::var(k)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let pairs: &[(&str, &str)] = match kind {
        "ai_claude" => &[("api_key", "ANTHROPIC_API_KEY"), ("model", "CLAUDE_MODEL")],
        "ai_muse" => &[
            ("api_key", "MODEL_API_KEY"),
            ("base_url", "MUSE_BASE_URL"),
            ("model", "MUSE_MODEL"),
        ],
        "ai_typesafe" => &[
            ("api_key", "TYPESAFE_API_KEY"),
            ("base_url", "TYPESAFE_BASE_URL"),
            ("model", "JEV_MODEL"),
        ],
        "ai_higgsfield" => &[
            ("api_key", "VIDEO_API_KEY"),
            ("base_url", "VIDEO_API_URL"),
            ("provider", "VIDEO_PROVIDER"),
        ],
        "voice" => &[("api_key", "VOICE_API_KEY"), ("base_url", "VOICE_API_URL")],
        "asr" => &[("api_key", "ASR_API_KEY"), ("base_url", "ASR_API_URL")],
        "whatsapp" => &[
            ("access_token", "WHATSAPP_ACCESS_TOKEN"),
            ("phone_number_id", "WHATSAPP_PHONE_NUMBER_ID"),
        ],
        _ => &[],
    };
    pairs
        .iter()
        .filter_map(|(k, e)| env(e).map(|v| ((*k).to_string(), Value::String(v))))
        .collect()
}

/// Required credentials are present and non-empty. Kinds with a vendor endpoint also need one.
pub fn has_credentials(kind: &str, config: &Map<String, Value>) -> bool {
    let Ok(k) = IntegrationKind::from_str(kind) else {
        return false;
    };
    let present = |key: &str| {
        config
            .get(key)
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.trim().is_empty())
    };
    let endpoint_ok = match kind {
        "ai_higgsfield" | "voice" | "asr" => present("base_url"),
        _ => true,
    };
    k.required_keys().iter().all(|key| present(key)) && endpoint_ok
}

/// A key the worker may use, with who pays for it.
#[derive(Debug, Clone)]
pub struct Credential {
    pub kind: &'static str,
    pub source: KeySource,
    pub config: Map<String, Value>,
    pub model: String,
    pub prices: Prices,
}

impl Credential {
    pub fn secret(&self, key: &str) -> String {
        self.config_str(key).unwrap_or_default()
    }

    pub fn config_str(&self, key: &str) -> Option<String> {
        self.config
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn base_url(&self, default: &str) -> String {
        self.config_str("base_url")
            .unwrap_or_else(|| default.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unresolved {
    pub reason: Unavailable,
    pub message: String,
}

impl Unresolved {
    /// Kept off by the plan or the budget: the slot must refuse rather than fall back.
    pub fn blocks(&self) -> bool {
        matches!(
            self.reason,
            Unavailable::NotInPlan | Unavailable::BudgetReached
        )
    }
}

/// Everything the worker needs to build one company's providers.
#[derive(Debug, Clone)]
pub struct ResolvedAi {
    pub routing: AiRouting,
    pub plan: AiPlan,
    pub platform_spend_this_month: f64,
    creds: HashMap<&'static str, Result<Credential, Unresolved>>,
}

impl ResolvedAi {
    pub fn get(&self, kind: &str) -> Result<Credential, Unresolved> {
        self.creds.get(kind).cloned().unwrap_or(Err(Unresolved {
            reason: Unavailable::NotConfigured,
            message: Unavailable::NotConfigured.message().into(),
        }))
    }
}

pub fn month_start(now: DateTime<Utc>) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .unwrap_or(now)
}

pub fn next_month_start(now: DateTime<Utc>) -> DateTime<Utc> {
    let (y, m) = if now.month() == 12 {
        (now.year() + 1, 1)
    } else {
        (now.year(), now.month() + 1)
    };
    Utc.with_ymd_and_hms(y, m, 1, 0, 0, 0)
        .single()
        .unwrap_or(now)
}

fn decrypt(state: &AppState, blob: Option<&[u8]>) -> Map<String, Value> {
    blob.and_then(|b| state.secrets.decrypt_json(b).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// The platform's config for every kind: the operator's settings over the environment.
pub async fn platform_configs(
    state: &AppState,
) -> Result<HashMap<&'static str, Map<String, Value>>, ApiError> {
    let mut tx = state.db.admin_tx().await?;
    let rows = repo::platform_integrations_admin(&mut tx).await?;
    tx.commit().await?;
    Ok(PROVIDER_KINDS
        .iter()
        .map(|kind| {
            let mut cfg = env_config(kind);
            if let Some(r) = rows.iter().find(|r| r.kind == *kind) {
                for (k, v) in decrypt(state, r.config_enc.as_deref()) {
                    cfg.insert(k, v);
                }
            }
            (*kind, cfg)
        })
        .collect())
}

/// The company's own config for every kind it has saved.
async fn company_configs(
    state: &AppState,
    tenant_id: Uuid,
) -> Result<
    (
        HashMap<String, Map<String, Value>>,
        TenantSettings,
        AiPlan,
        f64,
    ),
    ApiError,
> {
    let mut tx = state.db.tenant_tx(tenant_id).await?;
    let rows = integrations::list(tx.conn()).await?;
    let tenant = tenants::get(tx.conn(), tenant_id).await?;
    let plan = repo::plan(tx.conn(), tenant_id).await?;
    let spend = repo::platform_cost_since(tx.conn(), month_start(Utc::now())).await?;
    tx.commit().await?;
    let configs = rows
        .iter()
        .map(|r| {
            (
                r.kind.as_str().to_string(),
                decrypt(state, r.config_enc.as_deref()),
            )
        })
        .collect();
    Ok((
        configs,
        TenantSettings::from_json(&tenant.settings),
        plan,
        spend,
    ))
}

fn credential(
    kind: &'static str,
    source: KeySource,
    config: Map<String, Value>,
    platform_prices: Prices,
) -> Credential {
    let model = config
        .get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| default_model(kind).map(String::from))
        .unwrap_or_default();
    // A company sees its own vendor prices when it entered them; otherwise the platform's list prices.
    let prices = Prices::from_config(&Value::Object(config.clone())).or(platform_prices);
    Credential {
        kind,
        source,
        config,
        model,
        prices,
    }
}

pub async fn resolve(state: &AppState, tenant_id: Uuid) -> Result<ResolvedAi, ApiError> {
    let platform = platform_configs(state).await?;
    let (company, settings, plan, spend) = company_configs(state, tenant_id).await?;
    let mut creds = HashMap::new();
    for kind in PROVIDER_KINDS {
        let own = company.get(kind).cloned().unwrap_or_default();
        let plat = platform.get(kind).cloned().unwrap_or_default();
        let platform_prices =
            Prices::from_config(&Value::Object(plat.clone())).or(default_prices(kind));
        let resolved = match resolve_source(
            kind,
            has_credentials(kind, &own),
            has_credentials(kind, &plat),
            &plan,
            spend,
        ) {
            Ok(KeySource::Company) => {
                Ok(credential(kind, KeySource::Company, own, platform_prices))
            }
            Ok(KeySource::Platform) => {
                Ok(credential(kind, KeySource::Platform, plat, platform_prices))
            }
            Err(reason) => Err(Unresolved {
                message: reason.message().to_string(),
                reason,
            }),
        };
        creds.insert(kind, resolved);
    }
    Ok(ResolvedAi {
        routing: settings.ai,
        plan,
        platform_spend_this_month: spend,
        creds,
    })
}

/// What a company admin sees per provider: whose key is in use, never the key.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderAccess {
    pub kind: String,
    pub company_key: bool,
    pub platform_key: bool,
    /// `company`, `platform`, or null when unavailable.
    pub using: Option<KeySource>,
    pub unavailable: Option<Unavailable>,
    pub message: Option<String>,
    pub model: Option<String>,
}

pub async fn access(state: &AppState, tenant_id: Uuid) -> Result<Vec<ProviderAccess>, ApiError> {
    let platform = platform_configs(state).await?;
    let (company, _, _, _) = company_configs(state, tenant_id).await?;
    let resolved = resolve(state, tenant_id).await?;
    Ok(PROVIDER_KINDS
        .iter()
        .map(|kind| {
            let r = resolved.get(kind);
            ProviderAccess {
                kind: (*kind).to_string(),
                company_key: company.get(*kind).is_some_and(|c| has_credentials(kind, c)),
                platform_key: platform
                    .get(*kind)
                    .is_some_and(|c| has_credentials(kind, c)),
                using: r.as_ref().ok().map(|c| c.source),
                unavailable: r.as_ref().err().map(|e| e.reason.clone()),
                message: r.as_ref().err().map(|e| e.message.clone()),
                model: r
                    .as_ref()
                    .ok()
                    .map(|c| c.model.clone())
                    .filter(|m| !m.is_empty()),
            }
        })
        .collect())
}

/// One usage line, costed with the credential's prices.
#[allow(clippy::too_many_arguments)]
pub async fn record_usage(
    state: &AppState,
    tenant_id: Uuid,
    provider: &str,
    capability: &str,
    source: KeySource,
    prices: Prices,
    model: Option<&str>,
    input_tokens: u64,
    output_tokens: u64,
    units: u32,
    ok: bool,
    job_id: Option<Uuid>,
) -> Result<(), ApiError> {
    let cost = prices.cost(input_tokens, output_tokens, units);
    let mut tx = state.db.tenant_tx(tenant_id).await?;
    repo::insert_usage(
        tx.conn(),
        tenant_id,
        &repo::NewUsage {
            provider,
            capability,
            key_source: source.as_str(),
            model,
            input_tokens: i64::try_from(input_tokens).unwrap_or(i64::MAX),
            output_tokens: i64::try_from(output_tokens).unwrap_or(i64::MAX),
            units: i32::try_from(units).unwrap_or(i32::MAX),
            cost_usd: cost,
            ok,
            job_id,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// For the AI and messaging providers, only known options and credentials are stored; other
/// integrations (HR systems, SSO, storage) keep their own free-form settings.
pub fn allowed_key(kind: IntegrationKind, key: &str) -> bool {
    if kind_static(kind.as_str()).is_none() {
        return true;
    }
    OPTION_KEYS.contains(&key)
        || kind.secret_keys().contains(&key)
        || kind.required_keys().contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(v: Value) -> Map<String, Value> {
        v.as_object().cloned().unwrap()
    }

    #[test]
    fn credentials_need_every_required_key_and_an_endpoint_for_job_vendors() {
        assert!(has_credentials(
            "ai_typesafe",
            &obj(serde_json::json!({ "api_key": "k" }))
        ));
        assert!(!has_credentials(
            "ai_typesafe",
            &obj(serde_json::json!({ "api_key": " " }))
        ));
        assert!(!has_credentials(
            "voice",
            &obj(serde_json::json!({ "api_key": "k" }))
        ));
        assert!(has_credentials(
            "voice",
            &obj(serde_json::json!({ "api_key": "k", "base_url": "https://v" }))
        ));
        assert!(!has_credentials(
            "whatsapp",
            &obj(serde_json::json!({ "access_token": "t" }))
        ));
        assert!(!has_credentials("nope", &Map::new()));
    }

    #[test]
    fn credential_model_and_prices_fall_back() {
        let c = credential(
            "ai_typesafe",
            KeySource::Platform,
            obj(serde_json::json!({ "api_key": "k" })),
            Prices::jev(),
        );
        assert_eq!(c.model, DEFAULT_JEV_MODEL);
        assert_eq!(c.prices, Prices::jev());
        let own = credential(
            "ai_claude",
            KeySource::Company,
            obj(
                serde_json::json!({ "api_key": "k", "model": "claude-opus-5-5", "price_input_per_mtok": 5 }),
            ),
            Prices::default(),
        );
        assert_eq!(own.model, "claude-opus-5-5");
        assert_eq!(own.prices.input_per_mtok, Some(5.0));
    }

    #[test]
    fn months_roll_over() {
        let d = Utc.with_ymd_and_hms(2026, 12, 15, 10, 0, 0).unwrap();
        assert_eq!(
            month_start(d),
            Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap()
        );
        assert_eq!(
            next_month_start(d),
            Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap()
        );
    }
}
