//! The learner's minute: today's plan, watch, answer, progress, library, saved, requests, playback.

use crate::auth::AuthUser;
use crate::cache::keys;
use crate::dto::*;
use crate::error::ApiError;
use crate::idempotency;
use crate::routes::{page_params, Json, Path, Query};
use crate::services::{engine, learn, views};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::Router;
use reelearn_db::repos::{learning, reels, topics};
use reelearn_domain::cursor::{page, Cursor};
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me/today", get(today))
        .route("/me/watch", post(watch))
        .route("/me/answers", post(answer))
        .route("/me/progress", get(progress))
        .route("/me/library", get(library))
        .route("/me/saved/{reel_id}", put(save).delete(unsave))
        .route("/me/reel-requests", post(reel_request))
        .route("/reels/{reel_id}", get(playable))
}

async fn today(State(state): State<AppState>, auth: AuthUser) -> Result<Json<DailyPlan>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    let plan = learn::today(&state, &mut tx, &ctx).await?;
    tx.commit().await?;
    Ok(Json(plan))
}

/// Shared idempotent-POST wrapper: replay a stored response for the same key + payload.
pub(crate) async fn idempotent<F, Fut>(
    state: &AppState,
    auth: &AuthUser,
    headers: &HeaderMap,
    body: &Bytes,
    run: F,
) -> Result<Response, ApiError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(StatusCode, serde_json::Value), ApiError>>,
{
    let key = idempotency::key_from_headers(headers)?;
    let fp = idempotency::fingerprint(body);
    if let Some(k) = key.as_deref() {
        if let Some(replayed) = idempotency::replay(&state.cache, auth.tenant_id, k, &fp).await? {
            return Ok(replayed);
        }
    }
    let (status, value) = run().await?;
    if let Some(k) = key.as_deref() {
        idempotency::store(&state.cache, auth.tenant_id, k, &fp, status, &value).await;
    }
    Ok((status, axum::Json(value)).into_response())
}

pub(crate) fn parse_body<T: serde::de::DeserializeOwned>(body: &Bytes) -> Result<T, ApiError> {
    serde_json::from_slice(body)
        .map_err(|e| ApiError::BadRequest(format!("invalid JSON body: {e}")))
}

async fn watch(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let req: WatchRequest = parse_body(&body)?;
    if req.watched_ms < 0 {
        return Err(ApiError::field("watched_ms", "must be >= 0"));
    }
    let device_id = headers
        .get("x-device-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    idempotent(&state, &auth, &headers, &body, || async {
        let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
        let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
        learn::record_watch(&state, &mut tx, &ctx, &req, device_id.as_deref()).await?;
        tx.commit().await?;
        Ok((StatusCode::ACCEPTED, serde_json::json!({})))
    })
    .await
}

async fn answer(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let req: AnswerRequest = parse_body(&body)?;
    if req.chosen_key.trim().is_empty() {
        return Err(ApiError::field("chosen_key", "is required"));
    }
    idempotent(&state, &auth, &headers, &body, || async {
        let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
        let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
        let outcome = learn::record_answer(&state, &mut tx, &ctx, &req).await?;
        tx.commit().await?;
        Ok((StatusCode::OK, serde_json::to_value(outcome)?))
    })
    .await
}

async fn progress(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Progress>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    let p = learn::progress(&mut tx, &ctx).await?;
    tx.commit().await?;
    Ok(Json(p))
}

#[derive(Deserialize)]
struct LibraryQuery {
    topic_id: Option<Uuid>,
    q: Option<String>,
    saved: Option<bool>,
    #[serde(flatten)]
    page: PageQuery,
}

async fn library(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<LibraryQuery>,
) -> Result<Json<Page<ReelCard>>, ApiError> {
    let (cursor, limit) = page_params(&q.page)?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = reels::library(
        tx.conn(),
        auth.user_id,
        q.topic_id,
        q.q.as_deref(),
        q.saved.unwrap_or(false),
        cursor,
        limit,
    )
    .await?;
    let (rows, next) = page(rows, limit, |r| Cursor {
        ts: r.published_at,
        id: r.id,
    });
    let mut items = Vec::with_capacity(rows.len());
    for r in rows {
        items.push(ReelCard {
            id: r.id,
            title: r.title,
            topic_name: r.topic_name,
            presenter_name: r
                .presenter
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            duration_ms: r.duration_ms.unwrap_or(30_000),
            language: r.language,
            thumbnail_url: crate::services::media::signed_url(&state, r.thumbnail_key.as_deref())
                .await,
            saved: r.saved,
            seen: r.seen,
        });
    }
    tx.commit().await?;
    Ok(Json(Page {
        items,
        next_cursor: next,
    }))
}

async fn save(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reel_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    if !reelearn_db::repos::engine::may_play(
        tx.conn(),
        auth.user_id,
        reel_id,
        ctx.settings.engine.open_catalog,
    )
    .await?
    {
        return Err(ApiError::NotFound("reel not found".into()));
    }
    let was_saved = learning::is_saved(tx.conn(), auth.user_id, reel_id).await?;
    learning::save_reel(tx.conn(), auth.tenant_id, auth.user_id, reel_id).await?;
    if !was_saved {
        let reel = reels::get(tx.conn(), reel_id).await?;
        engine::apply_signal(
            &mut tx,
            &ctx,
            &reel,
            reelearn_domain::EngagementKind::Save,
            None,
            None,
            None,
            reelearn_domain::Channel::App,
        )
        .await?;
    }
    tx.commit().await?;
    state
        .cache
        .del_prefix(&keys::plan_prefix(auth.tenant_id, auth.user_id))
        .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn unsave(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reel_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    let was_saved = learning::is_saved(tx.conn(), auth.user_id, reel_id).await?;
    learning::unsave_reel(tx.conn(), auth.user_id, reel_id).await?;
    if was_saved {
        if let Some(reel) = reels::find(tx.conn(), reel_id).await? {
            engine::apply_signal(
                &mut tx,
                &ctx,
                &reel,
                reelearn_domain::EngagementKind::Unsave,
                None,
                None,
                None,
                reelearn_domain::Channel::App,
            )
            .await?;
        }
    }
    tx.commit().await?;
    state
        .cache
        .del_prefix(&keys::plan_prefix(auth.tenant_id, auth.user_id))
        .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn reel_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<ReelRequestCreate>,
) -> Result<StatusCode, ApiError> {
    let text = body.text.trim();
    if text.chars().count() < 3 || text.chars().count() > 500 {
        return Err(ApiError::field("text", "must be 3 to 500 characters"));
    }
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    learning::create_reel_request(tx.conn(), auth.tenant_id, auth.user_id, text).await?;
    tx.commit().await?;
    Ok(StatusCode::CREATED)
}

#[derive(Deserialize)]
struct LangQuery {
    lang: Option<String>,
}

const REEL_CACHE_TTL: Duration = Duration::from_secs(10 * 60);

async fn playable(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reel_id): Path<Uuid>,
    Query(q): Query<LangQuery>,
) -> Result<Json<ReelPlayable>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let ctx = learn::load_ctx(&mut tx, auth.user_id).await?;
    let lang = q
        .lang
        .unwrap_or_else(|| ctx.user.preferred_language.clone());
    if !ctx.user.role.at_least(reelearn_domain::Role::Admin)
        && !reelearn_db::repos::engine::may_play(
            tx.conn(),
            auth.user_id,
            reel_id,
            ctx.settings.engine.open_catalog,
        )
        .await?
    {
        return Err(ApiError::NotFound("reel not found".into()));
    }
    // Cached metadata (no media URLs, no per-user fields); per-user bits are re-applied below.
    let cache_key = keys::reel(reel_id, &lang);
    let cached: Option<ReelPlayable> = state.cache.get_json(&cache_key).await;
    let mut view = match cached {
        Some(v) => v,
        None => {
            let reel = reels::get(tx.conn(), reel_id).await?;
            let topic = topics::get(tx.conn(), reel.topic_id).await?;
            let v =
                views::reel_playable(&state, &mut tx, &ctx.user, &reel, &lang, &topic.name).await?;
            let mut to_cache = v.clone();
            to_cache.video_url = String::new();
            to_cache.captions_url = None;
            to_cache.thumbnail_url = None;
            to_cache.saved = false;
            to_cache.why_am_i_seeing_this = String::new();
            state
                .cache
                .set_json(&cache_key, &to_cache, REEL_CACHE_TTL)
                .await;
            v
        }
    };
    if view.video_url.is_empty() {
        let reel = reels::get(tx.conn(), reel_id).await?;
        let topic = topics::get(tx.conn(), reel.topic_id).await?;
        let fresh =
            views::reel_playable(&state, &mut tx, &ctx.user, &reel, &lang, &topic.name).await?;
        view.video_url = fresh.video_url;
        view.captions_url = fresh.captions_url;
        view.thumbnail_url = fresh.thumbnail_url;
        view.saved = fresh.saved;
        view.why_am_i_seeing_this = fresh.why_am_i_seeing_this;
    }
    view.captions_on_by_default = ctx.user.captions_on;
    tx.commit().await?;
    Ok(Json(view))
}
