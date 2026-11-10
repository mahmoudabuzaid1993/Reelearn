//! The learning engine over HTTP.
//!
//! Learner: `GET /me/next` (practice batches), `POST /me/events` (signals),
//! `POST /me/sessions/{id}/end`, `GET|PUT /me/topics` (the person's own follow/mute),
//! `GET /me/colleagues`, `POST /me/shares`.
//! Admin: `GET|PUT /admin/engine` (intensity and weights), `GET /admin/engine/insights`.

use crate::auth::AuthUser;
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::learn::{idempotent, parse_body};
use crate::routes::{range_window, record_audit, Json, Path, Query};
use crate::services::{engine, learn};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post, put};
use axum::Router;
use reelearn_db::repos::tenants;
use reelearn_db::settings::local_today;
use reelearn_db::TenantSettings;
use serde::Deserialize;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me/next", get(next))
        .route("/me/events", post(events))
        .route("/me/sessions/{id}/end", post(end_session))
        .route("/me/topics", get(my_topics))
        .route("/me/topics/{topic_id}", put(set_topic))
        .route("/me/personalization", put(set_personalization))
        .route("/me/colleagues", get(colleagues))
        .route("/me/shares", post(share))
}

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/admin/engine", get(get_engine).put(put_engine))
        .route("/admin/engine/insights", get(insights))
}

#[derive(Deserialize)]
struct NextQuery {
    session_id: Option<Uuid>,
    limit: Option<u8>,
}

async fn next(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<NextQuery>,
) -> Result<Json<NextBatch>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    let batch = engine::next(&state, &mut tx, &ctx, q.session_id, q.limit).await?;
    tx.commit().await?;
    Ok(Json(batch))
}

async fn events(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let req: EngagementBatch = parse_body(&body)?;
    idempotent(&state, &auth, &headers, &body, || async {
        let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
        let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
        let recorded = engine::record_events(&mut tx, &ctx, &req).await?;
        tx.commit().await?;
        Ok((
            StatusCode::ACCEPTED,
            serde_json::json!({ "recorded": recorded }),
        ))
    })
    .await
}

async fn end_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    engine::end_session(&mut tx, &ctx, id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn my_topics(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<MyInterests>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    let out = engine::my_interests(&mut tx, &ctx).await?;
    tx.commit().await?;
    Ok(Json(out))
}

async fn set_topic(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(topic_id): Path<Uuid>,
    Json(body): Json<TopicPreferenceUpdate>,
) -> Result<Json<MyInterests>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    engine::set_topic_preference(&mut tx, &ctx, topic_id, body.preference.as_deref()).await?;
    let out = engine::my_interests(&mut tx, &ctx).await?;
    tx.commit().await?;
    Ok(Json(out))
}

async fn set_personalization(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<PersonalizationUpdate>,
) -> Result<Json<MyInterests>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    reelearn_db::repos::users::set_personalized(tx.conn(), auth.user_id, body.enabled).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    let out = engine::my_interests(&mut tx, &ctx).await?;
    tx.commit().await?;
    Ok(Json(out))
}

#[derive(Deserialize)]
struct ColleagueQuery {
    q: Option<String>,
}

async fn colleagues(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ColleagueQuery>,
) -> Result<Json<Vec<Colleague>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    let out = engine::colleagues(&mut tx, &ctx, q.q.as_deref()).await?;
    tx.commit().await?;
    Ok(Json(out))
}

async fn share(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<ShareCreate>,
) -> Result<(StatusCode, Json<ShareResult>), ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    let out = engine::share(&mut tx, &ctx, &body).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(out)))
}

async fn get_engine(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<EngineConfig>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let t = tenants::get(tx.conn(), auth.tenant_id).await?;
    tx.commit().await?;
    Ok(Json(EngineConfig {
        settings: TenantSettings::from_json(&t.settings).engine,
        presets: engine::presets(),
    }))
}

async fn put_engine(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<EngineConfig>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let t = tenants::get(tx.conn(), auth.tenant_id).await?;
    let mut settings = TenantSettings::from_json(&t.settings);
    let before = settings.engine.clone();
    settings.engine = engine::apply_engine_patch(&before, &body)?;
    let updated = tenants::update(
        tx.conn(),
        auth.tenant_id,
        tenants::TenantPatch {
            name: None,
            languages: None,
            default_language: None,
            timezone: None,
            settings: Some(settings.to_json()),
            logo_key: None,
        },
    )
    .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "engine.update",
        "tenant",
        Some(updated.id),
        Some(serde_json::json!({ "before": before, "after": settings.engine })),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(EngineConfig {
        settings: settings.engine,
        presets: engine::presets(),
    }))
}

#[derive(Deserialize)]
struct InsightsQuery {
    range: Option<String>,
}

async fn insights(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<InsightsQuery>,
) -> Result<Json<EngineInsights>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let t = tenants::get(tx.conn(), auth.tenant_id).await?;
    let settings = TenantSettings::from_json(&t.settings);
    let today = local_today(&t.timezone, chrono::Utc::now());
    let (from, _, label) = range_window(q.range.as_deref(), today);
    let since = engine::local_day_start(&t.timezone, from);
    let out = engine::insights(&mut tx, &settings.engine, &t.timezone, since, label).await?;
    tx.commit().await?;
    Ok(Json(out))
}
