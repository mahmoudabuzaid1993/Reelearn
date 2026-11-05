//! `/sme/*`: phone recordings from subject-matter experts and managers.

use crate::auth::AuthUser;
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::{record_audit, Json, Path};
use crate::s3::{upload_key, UPLOAD_TTL};
use crate::services::views;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::Router;
use chrono::Utc;
use reelearn_db::repos::{media, reels, studio, topics, users};
use reelearn_domain::{ReelKind, ReelStatus, SourceKind};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sme/recordings", post(start_recording))
        .route(
            "/sme/recordings/{recording_id}/submit",
            post(submit_recording),
        )
}

const MAX_UPLOAD_BYTES: i64 = 209_715_200;

async fn start_recording(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<RecordingRequest>,
) -> Result<(StatusCode, Json<UploadSlot>), ApiError> {
    if !body.consent_confirmed {
        return Err(ApiError::field(
            "consent_confirmed",
            "face and voice consent must be confirmed before uploading",
        ));
    }
    if body.bytes <= 0 || body.bytes > MAX_UPLOAD_BYTES {
        return Err(ApiError::field("bytes", "must be between 1 and 200 MB"));
    }
    if !body.mime.starts_with("video/") && !body.mime.starts_with("audio/") {
        return Err(ApiError::field("mime", "must be a video or audio type"));
    }
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let topic = topics::get(tx.conn(), body.topic_id).await?;
    let asset_id = Uuid::now_v7();
    let ext = body.mime.split('/').nth(1).unwrap_or("bin");
    let key = upload_key(auth.tenant_id, asset_id, &format!("recording.{ext}"));
    let asset = media::create(
        tx.conn(),
        auth.tenant_id,
        &key,
        &body.mime,
        body.bytes,
        None,
        Some(auth.user_id),
    )
    .await?;
    let title = body
        .title
        .clone()
        .unwrap_or_else(|| format!("{} · recording", topic.name));
    let source = studio::create_source(
        tx.conn(),
        auth.tenant_id,
        SourceKind::SmeRecording,
        &title,
        None,
        Some(asset.id),
        None,
        None,
        auth.user_id,
    )
    .await?;
    // The recording id is the source id; the topic travels in the source prompt field until submit.
    sqlx::query("UPDATE sources SET prompt = $2 WHERE id = $1")
        .bind(source.id)
        .bind(body.topic_id.to_string())
        .execute(tx.conn())
        .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "recording.start",
        "source",
        Some(source.id),
        None,
    )
    .await?;
    tx.commit().await?;
    let mut headers = std::collections::BTreeMap::new();
    headers.insert("Content-Type".to_string(), body.mime.clone());
    Ok((
        StatusCode::CREATED,
        Json(UploadSlot {
            id: source.id,
            upload_url: state.storage.presign_put(&key, &body.mime),
            method: "PUT",
            headers,
            expires_at: Utc::now() + chrono::Duration::from_std(UPLOAD_TTL).unwrap_or_default(),
        }),
    ))
}

async fn submit_recording(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(recording_id): Path<Uuid>,
) -> Result<(StatusCode, Json<Reel>), ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let source = studio::get_source(tx.conn(), recording_id).await?;
    if source.kind != SourceKind::SmeRecording || source.created_by != auth.user_id {
        return Err(ApiError::NotFound("recording not found".into()));
    }
    let asset_id = source
        .media_asset_id
        .ok_or_else(|| ApiError::Conflict("recording has no upload".into()))?;
    let asset = media::get(tx.conn(), asset_id).await?;
    let topic_id: Uuid = source
        .prompt
        .as_deref()
        .and_then(|p| p.parse().ok())
        .ok_or_else(|| ApiError::Conflict("recording has no topic".into()))?;
    let user = users::get(tx.conn(), auth.user_id).await?;
    let reel = reels::create(
        tx.conn(),
        auth.tenant_id,
        reels::NewReel {
            topic_id,
            source_id: Some(source.id),
            generation_job_id: None,
            kind: ReelKind::SmeRecorded,
            status: ReelStatus::Draft,
            title: &source.title,
            language: &user.preferred_language,
            script: None,
            source_citation: Some(&format!("Recorded by {}", user.full_name)),
            presenter: serde_json::json!({ "name": user.full_name, "kind": "human" }),
            video_key: Some(&asset.storage_key),
            captions_key: None,
            thumbnail_key: None,
            duration_ms: None,
            compliance_tag: None,
            consent_confirmed: true,
            created_by: auth.user_id,
        },
    )
    .await?;
    studio::set_extraction_status(
        tx.conn(),
        source.id,
        reelearn_domain::ExtractionStatus::Done,
        None,
    )
    .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "recording.submit",
        "reel",
        Some(reel.id),
        None,
    )
    .await?;
    let view = views::reel_admin(&state, &mut tx, &reel).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(view)))
}
