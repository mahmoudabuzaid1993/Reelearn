//! AI Reel Studio: uploads, sources, moments, generation jobs (quote, run, SSE, retry, storyboard).

use crate::auth::AuthUser;
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::{page_params, record_audit, Json, Path, Query};
use crate::s3::{upload_key, UPLOAD_TTL};
use crate::services::views;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use futures::Stream;
use reelearn_db::repos::{jobs, media, reels, studio, tenants, topics};
use reelearn_db::TenantTx;
use reelearn_domain::cursor::{page, Cursor};
use reelearn_domain::pricing::{self, GenerationQuoteInput};
use reelearn_domain::{GenStep, JobStatus, ReelKind, ReelStatus, SourceKind};
use std::convert::Infallible;
use std::time::Duration;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/uploads", post(create_upload))
        .route("/admin/sources", get(list_sources).post(create_source))
        .route("/admin/sources/{source_id}", get(get_source))
        .route("/admin/sources/{source_id}/moments", get(moments))
        .route(
            "/admin/sources/{source_id}/moments/{moment_id}/cut",
            post(cut_moment),
        )
        .route("/admin/generation-jobs/quote", post(quote))
        .route("/admin/generation-jobs", get(list_jobs).post(create_job))
        .route("/admin/generation-jobs/{job_id}", get(get_job))
        .route("/admin/generation-jobs/{job_id}/retry", post(retry_job))
        .route(
            "/admin/generation-jobs/{job_id}/storyboard",
            get(storyboard),
        )
}

/// The SSE route is mounted without the global timeout layer.
pub fn sse_routes() -> Router<AppState> {
    Router::new().route("/admin/generation-jobs/{job_id}/events", get(events))
}

const MAX_UPLOAD_BYTES: i64 = 209_715_200;

async fn create_upload(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<UploadRequest>,
) -> Result<(StatusCode, Json<UploadSlot>), ApiError> {
    if body.bytes <= 0 || body.bytes > MAX_UPLOAD_BYTES {
        return Err(ApiError::field("bytes", "must be between 1 and 200 MB"));
    }
    if body.mime.trim().is_empty() || !body.mime.contains('/') {
        return Err(ApiError::field("mime", "must be a media type"));
    }
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    if let Some(sha) = body.sha256.as_deref() {
        if let Some(existing) = media::find_by_sha(tx.conn(), sha).await? {
            // Identical upload already on file: hand back the same slot id, the PUT is harmless.
            let mut headers = std::collections::BTreeMap::new();
            headers.insert("Content-Type".to_string(), existing.mime.clone());
            return Ok((
                StatusCode::CREATED,
                Json(UploadSlot {
                    id: existing.id,
                    upload_url: state
                        .storage
                        .presign_put(&existing.storage_key, &existing.mime),
                    method: "PUT",
                    headers,
                    expires_at: Utc::now()
                        + chrono::Duration::from_std(UPLOAD_TTL).unwrap_or_default(),
                }),
            ));
        }
    }
    let asset_id = Uuid::now_v7();
    let key = upload_key(auth.tenant_id, asset_id, &body.filename);
    let asset = media::create(
        tx.conn(),
        auth.tenant_id,
        &key,
        &body.mime,
        body.bytes,
        body.sha256.as_deref(),
        Some(auth.user_id),
    )
    .await?;
    tx.commit().await?;
    let mut headers = std::collections::BTreeMap::new();
    headers.insert("Content-Type".to_string(), body.mime.clone());
    Ok((
        StatusCode::CREATED,
        Json(UploadSlot {
            id: asset.id,
            upload_url: state.storage.presign_put(&key, &body.mime),
            method: "PUT",
            headers,
            expires_at: Utc::now() + chrono::Duration::from_std(UPLOAD_TTL).unwrap_or_default(),
        }),
    ))
}

async fn source_view(tx: &mut TenantTx, s: &studio::SourceRow) -> Result<Source, ApiError> {
    let summary = studio::extraction(tx.conn(), s.id).await?.map(|e| {
        let mut v = e.summary.clone();
        if let Some(o) = v.as_object_mut() {
            o.entry("word_count")
                .or_insert(serde_json::json!(e.word_count));
        }
        v
    });
    Ok(Source {
        id: s.id,
        kind: s.kind,
        title: s.title.clone(),
        url: s.url.clone(),
        language: s.language.clone(),
        extraction_status: s.extraction_status,
        extraction_error: s.extraction_error.clone(),
        summary,
        created_by: s.created_by,
        created_at: s.created_at,
    })
}

async fn list_sources(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<PageQuery>,
) -> Result<Json<Page<Source>>, ApiError> {
    let (cursor, limit) = page_params(&q)?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = studio::list_sources(tx.conn(), cursor, limit).await?;
    let (rows, next) = page(rows, limit, |r| Cursor {
        ts: r.created_at,
        id: r.id,
    });
    let mut items = Vec::with_capacity(rows.len());
    for r in &rows {
        items.push(source_view(&mut tx, r).await?);
    }
    tx.commit().await?;
    Ok(Json(Page {
        items,
        next_cursor: next,
    }))
}

async fn create_source(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<SourceCreate>,
) -> Result<(StatusCode, Json<Source>), ApiError> {
    if body.title.trim().is_empty() {
        return Err(ApiError::field("title", "is required"));
    }
    match body.kind {
        SourceKind::Url | SourceKind::Youtube => {
            let url = body.url.as_deref().unwrap_or("");
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(ApiError::field("url", "must be an http(s) URL"));
            }
        }
        SourceKind::TopicPrompt => {
            if body
                .prompt
                .as_deref()
                .map(|p| p.trim().is_empty())
                .unwrap_or(true)
            {
                return Err(ApiError::field("prompt", "is required for a topic prompt"));
            }
        }
        SourceKind::SmeRecording => {
            return Err(ApiError::field(
                "kind",
                "recordings are created through /sme/recordings",
            ))
        }
        _ => {
            if body.media_asset_id.is_none() {
                return Err(ApiError::field(
                    "media_asset_id",
                    "is required for uploaded files",
                ));
            }
        }
    }
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    if let Some(asset) = body.media_asset_id {
        media::get(tx.conn(), asset).await?;
    }
    let s = studio::create_source(
        tx.conn(),
        auth.tenant_id,
        body.kind,
        body.title.trim(),
        body.url.as_deref(),
        body.media_asset_id,
        body.prompt.as_deref(),
        body.language.as_deref(),
        auth.user_id,
    )
    .await?;
    let kind = if body.kind.is_audio_visual() {
        "transcribe_podcast"
    } else {
        "extract_source"
    };
    jobs::enqueue(
        tx.conn(),
        Some(auth.tenant_id),
        kind,
        serde_json::json!({ "source_id": s.id }),
        None,
    )
    .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "source.create",
        "source",
        Some(s.id),
        Some(serde_json::json!({ "kind": s.kind })),
    )
    .await?;
    let view = source_view(&mut tx, &s).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(view)))
}

async fn get_source(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(source_id): Path<Uuid>,
) -> Result<Json<Source>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let s = studio::get_source(tx.conn(), source_id).await?;
    let view = source_view(&mut tx, &s).await?;
    tx.commit().await?;
    Ok(Json(view))
}

fn moment_view(m: &studio::MomentRow) -> Moment {
    Moment {
        id: m.id,
        start_ms: m.start_ms,
        end_ms: m.end_ms,
        excerpt: m.excerpt.clone(),
        title: m.title.clone(),
        speaker: m.speaker.clone(),
        score: m.score,
        selected: m.selected,
        reel_id: m.reel_id,
    }
}

async fn moments(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(source_id): Path<Uuid>,
) -> Result<Json<Vec<Moment>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    studio::get_source(tx.conn(), source_id).await?;
    let rows = studio::moments(tx.conn(), source_id).await?;
    tx.commit().await?;
    Ok(Json(rows.iter().map(moment_view).collect()))
}

async fn cut_moment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((source_id, moment_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CutRequest>,
) -> Result<(StatusCode, Json<Reel>), ApiError> {
    if !body.speaker_consent {
        return Err(ApiError::field(
            "speaker_consent",
            "the speaker's consent is required before cutting a reel",
        ));
    }
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let source = studio::get_source(tx.conn(), source_id).await?;
    let moment = studio::moment(tx.conn(), moment_id).await?;
    if moment.source_id != source.id {
        return Err(ApiError::NotFound("moment not found".into()));
    }
    if let Some(existing) = moment.reel_id {
        return Err(ApiError::Conflict(format!(
            "this moment was already cut into reel {existing}"
        )));
    }
    let topic = topics::get(tx.conn(), body.topic_id).await?;
    let start = body.start_ms.unwrap_or(moment.start_ms).max(0);
    let end = body.end_ms.unwrap_or(moment.end_ms);
    if end <= start || end - start > 90_000 {
        return Err(ApiError::field(
            "end_ms",
            "a moment must be between 1 and 90 seconds",
        ));
    }
    tenants::spend_credits(tx.conn(), auth.tenant_id, pricing::CUT_PER_MOMENT).await?;
    let title = body
        .title
        .clone()
        .or_else(|| moment.title.clone())
        .unwrap_or_else(|| format!("{} · moment", source.title));
    let reel = reels::create(
        tx.conn(),
        auth.tenant_id,
        reels::NewReel {
            topic_id: topic.id,
            source_id: Some(source.id),
            generation_job_id: None,
            kind: ReelKind::PodcastCut,
            status: ReelStatus::Draft,
            title: &title,
            language: source.language.as_deref().unwrap_or("ar"),
            script: Some(&moment.excerpt),
            source_citation: Some(&source.title),
            presenter: serde_json::json!({ "name": moment.speaker, "kind": "human" }),
            video_key: None,
            captions_key: None,
            thumbnail_key: None,
            duration_ms: Some((end - start).clamp(1000, 120_000)),
            compliance_tag: topic.compliance_tag.as_deref(),
            consent_confirmed: true,
            created_by: auth.user_id,
        },
    )
    .await?;
    studio::mark_moment_cut(
        tx.conn(),
        moment.id,
        reel.id,
        start,
        end,
        body.title.as_deref(),
    )
    .await?;
    jobs::enqueue(tx.conn(), Some(auth.tenant_id), GenStep::Cut.job_kind(), serde_json::json!({ "reel_id": reel.id, "source_id": source.id, "moment_id": moment.id, "start_ms": start, "end_ms": end }), None).await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "moment.cut",
        "reel",
        Some(reel.id),
        None,
    )
    .await?;
    let view = views::reel_admin(&state, &mut tx, &reel).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(view)))
}

fn validate_request(req: &GenerationRequest, tenant_languages: &[String]) -> Result<(), ApiError> {
    if !(1..=12).contains(&req.reel_count) {
        return Err(ApiError::field("reel_count", "must be between 1 and 12"));
    }
    if req.languages.is_empty() {
        return Err(ApiError::field(
            "languages",
            "must contain at least one language",
        ));
    }
    for l in &req.languages {
        if !tenant_languages.iter().any(|t| t == l) {
            return Err(ApiError::field(
                "languages",
                &format!("{l} is not one of the workspace languages"),
            ));
        }
    }
    Ok(())
}

async fn quote(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<GenerationRequest>,
) -> Result<Json<QuoteResponse>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    validate_request(&body, &tenant.languages)?;
    studio::get_source(tx.conn(), body.source_id).await?;
    let needs_extraction = studio::extraction(tx.conn(), body.source_id)
        .await?
        .is_none();
    tx.commit().await?;
    let q = pricing::quote_generation(&GenerationQuoteInput {
        reel_count: body.reel_count,
        language_count: i32::try_from(body.languages.len()).unwrap_or(1),
        style: body.style,
        needs_extraction,
    });
    Ok(Json(QuoteResponse {
        cost_credits: q.cost_credits,
        credits_available: tenant.credits,
        estimated_minutes: q.estimated_minutes,
        breakdown: q.breakdown,
    }))
}

pub async fn job_view(
    tx: &mut TenantTx,
    j: &studio::GenerationJobRow,
) -> Result<GenerationJob, ApiError> {
    let steps = studio::steps(tx.conn(), j.id)
        .await?
        .into_iter()
        .map(|s| GenerationStep {
            position: s.position,
            step: s.step,
            status: s.status,
            provider: s.provider,
            attempt: s.attempt,
            started_at: s.started_at,
            finished_at: s.finished_at,
            log: s.log,
        })
        .collect();
    let reel_ids = reels::for_job(tx.conn(), j.id)
        .await?
        .into_iter()
        .map(|r| r.id)
        .collect();
    Ok(GenerationJob {
        id: j.id,
        source_id: j.source_id,
        topic_id: j.param_uuid("topic_id"),
        status: j.status,
        progress_pct: j.progress_pct,
        cost_credits: j.cost_credits,
        params: j.params.clone(),
        error: j.error.clone(),
        steps,
        reel_ids,
        created_at: j.created_at,
        finished_at: j.finished_at,
    })
}

async fn list_jobs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<PageQuery>,
) -> Result<Json<Page<GenerationJob>>, ApiError> {
    let (cursor, limit) = page_params(&q)?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = studio::list_jobs(tx.conn(), cursor, limit).await?;
    let (rows, next) = page(rows, limit, |r| Cursor {
        ts: r.created_at,
        id: r.id,
    });
    let mut items = Vec::with_capacity(rows.len());
    for r in &rows {
        items.push(job_view(&mut tx, r).await?);
    }
    tx.commit().await?;
    Ok(Json(Page {
        items,
        next_cursor: next,
    }))
}

async fn create_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<GenerationRequest>,
) -> Result<(StatusCode, Json<GenerationJob>), ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    validate_request(&body, &tenant.languages)?;
    let source = studio::get_source(tx.conn(), body.source_id).await?;
    topics::get(tx.conn(), body.topic_id).await?;
    if source.kind.is_audio_visual() {
        return Err(ApiError::UnprocessableEntity(
            "podcasts and talks are cut from moments, not scripted; use the moments endpoint"
                .into(),
        ));
    }
    let needs_extraction = studio::extraction(tx.conn(), body.source_id)
        .await?
        .is_none();
    let q = pricing::quote_generation(&GenerationQuoteInput {
        reel_count: body.reel_count,
        language_count: i32::try_from(body.languages.len()).unwrap_or(1),
        style: body.style,
        needs_extraction,
    });
    tenants::spend_credits(tx.conn(), auth.tenant_id, q.cost_credits).await?;
    let params = serde_json::json!({
        "kind": "generate",
        "topic_id": body.topic_id,
        "languages": body.languages,
        "reel_count": body.reel_count,
        "style": body.style,
        "presenter": body.presenter.clone().unwrap_or_else(|| serde_json::json!({ "name": "Reelearn presenter", "kind": "ai" })),
        "brief": body.brief,
        "quote": q,
    });
    let job = studio::create_job(
        tx.conn(),
        auth.tenant_id,
        source.id,
        auth.user_id,
        body.reviewer_id,
        &params,
        q.cost_credits,
    )
    .await?;
    let steps = GenStep::full_pipeline(body.languages.len() > 1);
    studio::insert_steps(tx.conn(), auth.tenant_id, job.id, &steps).await?;
    jobs::enqueue(
        tx.conn(),
        Some(auth.tenant_id),
        steps[0].job_kind(),
        serde_json::json!({ "generation_job_id": job.id, "position": 0, "source_id": source.id }),
        None,
    )
    .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "generation.start",
        "generation_job",
        Some(job.id),
        Some(serde_json::json!({ "cost_credits": q.cost_credits })),
    )
    .await?;
    let view = job_view(&mut tx, &job).await?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(view)))
}

async fn get_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(job_id): Path<Uuid>,
) -> Result<Json<GenerationJob>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let j = studio::get_job(tx.conn(), job_id).await?;
    let view = job_view(&mut tx, &j).await?;
    tx.commit().await?;
    Ok(Json(view))
}

async fn snapshot(
    state: &AppState,
    tenant_id: Uuid,
    job_id: Uuid,
) -> Result<GenerationJob, ApiError> {
    let mut tx = state.db.tenant_tx(tenant_id).await?;
    let j = studio::get_job(tx.conn(), job_id).await?;
    let view = job_view(&mut tx, &j).await?;
    tx.commit().await?;
    Ok(view)
}

async fn events(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(job_id): Path<Uuid>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let first = snapshot(&state, auth.tenant_id, job_id).await?;
    let stream = async_stream::stream! {
        let mut last = serde_json::to_string(&first).unwrap_or_default();
        yield Ok(Event::default().event("snapshot").data(last.clone()));
        if first.status.is_terminal() {
            return;
        }
        let mut ticks = 0u32;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            ticks += 1;
            match snapshot(&state, auth.tenant_id, job_id).await {
                Ok(snap) => {
                    let json = serde_json::to_string(&snap).unwrap_or_default();
                    if json != last {
                        last = json.clone();
                        yield Ok(Event::default().event("snapshot").data(json));
                    }
                    if snap.status.is_terminal() {
                        yield Ok(Event::default().event("end").data(snap.status.to_string()));
                        return;
                    }
                }
                Err(e) => {
                    yield Ok(Event::default().event("error").data(e.to_string()));
                    return;
                }
            }
            if ticks > 60 * 30 {
                return; // 30 minutes: the client reconnects (EventSource does so automatically)
            }
        }
    };
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    ))
}

async fn retry_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(job_id): Path<Uuid>,
) -> Result<(StatusCode, Json<GenerationJob>), ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let j = studio::get_job(tx.conn(), job_id).await?;
    if !matches!(j.status, JobStatus::Failed | JobStatus::Paused) {
        return Err(ApiError::Conflict(format!(
            "job is {}; only failed or paused jobs can be retried",
            j.status
        )));
    }
    if jobs::active_for_generation(tx.conn(), j.id).await? {
        return Err(ApiError::Conflict("a retry is already queued".into()));
    }
    let steps = studio::steps(tx.conn(), j.id).await?;
    let target = steps
        .iter()
        .find(|s| {
            matches!(
                s.status,
                JobStatus::Failed | JobStatus::Paused | JobStatus::Running
            )
        })
        .or_else(|| steps.iter().find(|s| s.status == JobStatus::Queued))
        .ok_or_else(|| ApiError::Conflict("nothing to retry".into()))?;
    studio::update_step(
        tx.conn(),
        target.id,
        JobStatus::Queued,
        None,
        None,
        Some("retry requested"),
        false,
    )
    .await?;
    studio::set_job_status(tx.conn(), j.id, JobStatus::Queued, None, None).await?;
    jobs::enqueue(tx.conn(), Some(auth.tenant_id), target.step.job_kind(), serde_json::json!({ "generation_job_id": j.id, "position": target.position, "source_id": j.source_id }), None).await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "generation.retry",
        "generation_job",
        Some(j.id),
        Some(serde_json::json!({ "step": target.step })),
    )
    .await?;
    let j = studio::get_job(tx.conn(), job_id).await?;
    let view = job_view(&mut tx, &j).await?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(view)))
}

async fn storyboard(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(job_id): Path<Uuid>,
) -> Result<Json<Vec<Reel>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    studio::get_job(tx.conn(), job_id).await?;
    let rows = reels::for_job(tx.conn(), job_id).await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        out.push(views::reel_admin(&state, &mut tx, r).await?);
    }
    tx.commit().await?;
    Ok(Json(out))
}
