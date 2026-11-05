//! `/admin/reports/*` and `/admin/exports`: aggregates only (no per-answer data), cached 60 s.

use crate::auth::AuthUser;
use crate::cache::keys;
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::me::export_view;
use crate::routes::{range_window, record_audit, Json, Path, Query};
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use reelearn_db::repos::{exports, jobs, stats, tenants, users};
use reelearn_db::settings::local_today;
use reelearn_domain::{ExportKind, JobStatus};
use std::time::Duration;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/reports/overview", get(overview))
        .route("/admin/reports/by-topic", get(by_topic))
        .route("/admin/reports/by-group", get(by_group))
        .route("/admin/exports", get(list_exports).post(create_export))
        .route("/admin/exports/{export_id}/download", get(download))
}

const REPORT_TTL: Duration = Duration::from_secs(60);

async fn overview(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<RangeQuery>,
) -> Result<Json<ReportOverview>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    let (from, to, label) = range_window(
        q.range.as_deref(),
        local_today(&tenant.timezone, Utc::now()),
    );
    let key = keys::report(auth.tenant_id, "overview", &label);
    if let Some(v) = state.cache.get_json::<ReportOverview>(&key).await {
        return Ok(Json(v));
    }
    let o = stats::overview(tx.conn(), from, to).await?;
    let learners_active = users::count_active_since(tx.conn(), from).await?;
    let minutes_on_mandatory = stats::total_minutes_on_mandatory(tx.conn()).await?;
    tx.commit().await?;
    let v = ReportOverview {
        learners_active,
        reels_completed: o.reels_completed,
        answers_total: o.answers_total,
        recall_pct: if o.answers_total > 0 {
            i32::try_from(100 * o.answers_correct / o.answers_total).unwrap_or(0)
        } else {
            0
        },
        minutes_on_mandatory: (minutes_on_mandatory * 100.0).round() / 100.0,
    };
    state.cache.set_json(&key, &v, REPORT_TTL).await;
    Ok(Json(v))
}

fn row_view(r: stats::ReportRowRaw) -> ReportRow {
    ReportRow {
        id: r.id,
        name: r.name,
        learners_active: r.learners_active,
        learners_total: r.learners_total,
        recall_pct: r.recall_pct,
        reels_completed: r.reels_completed,
        minutes_total: (r.minutes_total * 100.0).round() / 100.0,
        ready_pct: r.ready_pct,
    }
}

async fn by_topic(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Vec<ReportRow>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    let (from, to, label) = range_window(
        q.range.as_deref(),
        local_today(&tenant.timezone, Utc::now()),
    );
    let key = keys::report(auth.tenant_id, "by-topic", &label);
    if let Some(v) = state.cache.get_json::<Vec<ReportRow>>(&key).await {
        return Ok(Json(v));
    }
    let rows: Vec<ReportRow> = stats::report_by_topic(tx.conn(), from, to)
        .await?
        .into_iter()
        .map(row_view)
        .collect();
    tx.commit().await?;
    state.cache.set_json(&key, &rows, REPORT_TTL).await;
    Ok(Json(rows))
}

async fn by_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Vec<ReportRow>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    let (from, to, label) = range_window(
        q.range.as_deref(),
        local_today(&tenant.timezone, Utc::now()),
    );
    let key = keys::report(auth.tenant_id, "by-group", &label);
    if let Some(v) = state.cache.get_json::<Vec<ReportRow>>(&key).await {
        return Ok(Json(v));
    }
    let rows: Vec<ReportRow> = stats::report_by_group(tx.conn(), from, to)
        .await?
        .into_iter()
        .map(row_view)
        .collect();
    tx.commit().await?;
    state.cache.set_json(&key, &rows, REPORT_TTL).await;
    Ok(Json(rows))
}

async fn list_exports(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Export>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = exports::list(tx.conn()).await?;
    tx.commit().await?;
    Ok(Json(rows.iter().map(export_view).collect()))
}

async fn create_export(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<ExportRequest>,
) -> Result<(StatusCode, Json<Export>), ApiError> {
    if body.kind != ExportKind::HrdfEvidence {
        return Err(ApiError::field("kind", "must be hrdf_evidence"));
    }
    if body.period_end < body.period_start {
        return Err(ApiError::field(
            "period_end",
            "must be on or after period_start",
        ));
    }
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let row = exports::create(
        tx.conn(),
        auth.tenant_id,
        ExportKind::HrdfEvidence,
        auth.user_id,
        None,
        Some(body.period_start),
        Some(body.period_end),
    )
    .await?;
    jobs::enqueue(
        tx.conn(),
        Some(auth.tenant_id),
        "hrdf_export",
        serde_json::json!({ "export_id": row.id }),
        None,
    )
    .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "export.create",
        "export",
        Some(row.id),
        Some(
            serde_json::json!({ "period_start": body.period_start, "period_end": body.period_end }),
        ),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(export_view(&row))))
}

#[derive(serde::Serialize)]
struct Download {
    url: String,
    expires_at: chrono::DateTime<Utc>,
}

async fn download(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(export_id): Path<Uuid>,
) -> Result<Json<Download>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let row = exports::get(tx.conn(), export_id).await?;
    tx.commit().await?;
    if row.status != JobStatus::Done {
        return Err(ApiError::Conflict(format!("export is {}", row.status)));
    }
    let key = row
        .file_key
        .ok_or_else(|| ApiError::Conflict("export has no file".into()))?;
    Ok(Json(Download {
        url: state.storage.presign_get(&key),
        expires_at: Utc::now()
            + chrono::Duration::from_std(crate::s3::DOWNLOAD_TTL).unwrap_or_default(),
    }))
}
