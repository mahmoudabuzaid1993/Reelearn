//! Workspace creation (A01), settings, dashboard (A04).

use crate::auth::AuthUser;
use crate::cache::keys;
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::{range_window, record_audit, Json, Query};
use crate::services::{otp, views};
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use reelearn_db::repos::{reels, stats, tenants, users};
use reelearn_db::settings::{local_today, TenantSettings};
use reelearn_domain::{phone, Channel, DeliverySlot, Role, UserStatus};
use std::time::Duration;

pub fn public_routes() -> Router<AppState> {
    Router::new().route("/tenants", post(create_tenant))
}

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/admin/tenant", get(get_tenant).patch(patch_tenant))
        .route("/admin/dashboard", get(dashboard))
}

async fn create_tenant(
    State(state): State<AppState>,
    Json(body): Json<TenantCreate>,
) -> Result<(StatusCode, Json<TenantCreated>), ApiError> {
    let slug = body.slug.trim().to_ascii_lowercase();
    let slug_ok = slug.len() >= 2
        && slug.len() <= 63
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !slug.starts_with('-');
    if !slug_ok {
        return Err(ApiError::field(
            "slug",
            "must be 2-63 lowercase letters, digits or dashes",
        ));
    }
    if !matches!(body.default_language.as_str(), "ar" | "en") {
        return Err(ApiError::field("default_language", "must be ar or en"));
    }
    let email = body.admin_email.trim().to_ascii_lowercase();
    if !email.contains('@') {
        return Err(ApiError::field("admin_email", "must be an email address"));
    }
    let admin_phone = match body.admin_phone.as_deref().filter(|p| !p.trim().is_empty()) {
        Some(p) => {
            Some(phone::normalize(p).map_err(|e| ApiError::field("admin_phone", &e.to_string()))?)
        }
        None => None,
    };
    let languages = if body.default_language == "ar" {
        vec!["ar".to_string(), "en".to_string()]
    } else {
        vec!["en".to_string(), "ar".to_string()]
    };
    let timezone = match body.region {
        reelearn_domain::Region::Ksa => "Asia/Riyadh",
        reelearn_domain::Region::Uae => "Asia/Dubai",
        reelearn_domain::Region::Eu => "Europe/Berlin",
    };
    let mut settings = TenantSettings::default().to_json();
    if let (Some(obj), Some(sector)) = (settings.as_object_mut(), body.sector.as_deref()) {
        obj.insert(
            "sector".into(),
            serde_json::Value::String(sector.to_string()),
        );
    }

    let tenant = {
        let mut tx = state.db.admin_tx().await?;
        if tenants::find_by_slug(&mut tx, &slug).await?.is_some() {
            return Err(ApiError::Conflict(
                "a workspace with this slug already exists".into(),
            ));
        }
        let t = tenants::create(
            &mut tx,
            tenants::NewTenant {
                slug: &slug,
                name: body.company_name.trim(),
                region: body.region,
                default_language: &body.default_language,
                languages: &languages,
                timezone,
                settings,
                credits: 100,
            },
        )
        .await?;
        tx.commit().await?;
        t
    };

    let mut tx = state.db.tenant_tx(tenant.id).await?;
    let admin = users::create(
        tx.conn(),
        tenant.id,
        users::NewUser {
            role: Role::Admin,
            status: UserStatus::Invited,
            full_name: body.admin_name.trim(),
            phone_e164: admin_phone.as_deref(),
            email: Some(&email),
            employee_no: None,
            preferred_language: &body.default_language,
            ui_language: &body.default_language,
            manager_id: None,
            location: None,
            job_title: None,
            delivery_channel: Channel::App,
            delivery_slot: DeliverySlot::ShiftStart,
        },
    )
    .await?;
    let challenge = otp::create(&state, &mut tx, Some(admin.id), None, Some(&email)).await?;
    record_audit(
        &mut tx,
        admin.id,
        "tenant.create",
        "tenant",
        Some(tenant.id),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(TenantCreated {
            tenant: views::tenant_public(&tenant, None),
            challenge_id: challenge.id,
        }),
    ))
}

async fn get_tenant(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Tenant>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let t = tenants::get(tx.conn(), auth.tenant_id).await?;
    tx.commit().await?;
    let public = views::tenant_public_signed(&state, &t).await;
    Ok(Json(views::tenant_full(&t, public)))
}

async fn patch_tenant(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<TenantPatch>,
) -> Result<Json<Tenant>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let current = tenants::get(tx.conn(), auth.tenant_id).await?;
    if let Some(tz) = body.timezone.as_deref() {
        if tz.parse::<chrono_tz::Tz>().is_err() {
            return Err(ApiError::field("timezone", "is not a known IANA timezone"));
        }
    }
    if let Some(langs) = &body.languages {
        if langs.is_empty() {
            return Err(ApiError::field(
                "languages",
                "must contain at least one language",
            ));
        }
    }
    let settings = body.settings.as_ref().map(|p| {
        TenantSettings::from_json(&current.settings)
            .merge_patch(p)
            .to_json()
    });
    let t = tenants::update(
        tx.conn(),
        auth.tenant_id,
        tenants::TenantPatch {
            name: body.name,
            languages: body.languages,
            default_language: body.default_language,
            timezone: body.timezone,
            settings,
            logo_key: None,
        },
    )
    .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "tenant.update",
        "tenant",
        Some(t.id),
        Some(serde_json::json!({ "settings": t.settings })),
    )
    .await?;
    tx.commit().await?;
    let public = views::tenant_public_signed(&state, &t).await;
    Ok(Json(views::tenant_full(&t, public)))
}

async fn dashboard(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Dashboard>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    let today = local_today(&tenant.timezone, Utc::now());
    let (from, to, label) = range_window(q.range.as_deref(), today);
    let cache_key = keys::dash(auth.tenant_id, &label);
    if let Some(d) = state.cache.get_json::<Dashboard>(&cache_key).await {
        return Ok(Json(d));
    }
    let learners_total = users::count_learners(tx.conn()).await?;
    let learners_active = users::count_active_since(tx.conn(), from).await?;
    let weekly_active =
        users::count_active_since(tx.conn(), today - chrono::Duration::days(6)).await?;
    let overview = stats::overview(tx.conn(), from, to).await?;
    let recall_pct = if overview.answers_total > 0 {
        i32::try_from(100 * overview.answers_correct / overview.answers_total).unwrap_or(0)
    } else {
        0
    };
    let hrdf_hours = stats::total_minutes_on_mandatory(tx.conn()).await? / 60.0;
    let series = stats::day_series(tx.conn(), from, to)
        .await?
        .into_iter()
        .map(|d| DashboardPoint {
            day: d.day,
            active: d.active,
            recall_pct: if d.answers_total > 0 {
                i32::try_from(100 * d.answers_correct / d.answers_total).unwrap_or(0)
            } else {
                0
            },
        })
        .collect();
    let mut attention = Vec::new();
    let awaiting = reels::count_awaiting_review(tx.conn()).await?;
    if awaiting > 0 {
        attention.push(Attention {
            kind: "review".into(),
            text: format!("{awaiting} reel(s) awaiting sign-off"),
            href: "/library?status=in_review".into(),
        });
    }
    for u in stats::inactive_learners(tx.conn(), today - chrono::Duration::days(3), 5).await? {
        attention.push(Attention {
            kind: "inactive".into(),
            text: format!("{} has not learned for 3+ days", u.full_name),
            href: format!("/people/{}", u.user_id),
        });
    }
    if tenant.credits < 100 {
        attention.push(Attention {
            kind: "credits".into(),
            text: format!("{} credits left", tenant.credits),
            href: "/settings#credits".into(),
        });
    }
    tx.commit().await?;
    let d = Dashboard {
        weekly_active_pct: if learners_total > 0 {
            i32::try_from(100 * weekly_active / learners_total).unwrap_or(0)
        } else {
            0
        },
        recall_pct,
        learners_total,
        learners_active,
        hrdf_hours: (hrdf_hours * 100.0).round() / 100.0,
        credits: tenant.credits,
        attention,
        series,
    };
    state
        .cache
        .set_json(&cache_key, &d, Duration::from_secs(60))
        .await;
    Ok(Json(d))
}
