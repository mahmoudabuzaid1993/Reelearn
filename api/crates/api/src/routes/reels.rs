//! Reel library and lifecycle: edit, regenerate, variants, review, dual approval, publish, archive.

use crate::auth::AuthUser;
use crate::cache::keys;
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::studio::job_view;
use crate::routes::{page_params, record_audit, Json, Path, Query};
use crate::services::views;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use reelearn_db::repos::{groups, jobs, plans, reels, studio, tenants, topics, users};
use reelearn_db::settings::local_today;
use reelearn_db::TenantTx;
use reelearn_domain::cursor::{page, Cursor};
use reelearn_domain::pricing;
use reelearn_domain::{ApprovalRole, Decision, GenStep, GenerationStyle, ReelKind, ReelStatus};
use serde::Deserialize;
use uuid::Uuid;

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/admin/reels", get(list))
        .route("/admin/reels/{reel_id}", axum::routing::patch(patch))
        .route("/admin/reels/{reel_id}/regenerate", post(regenerate))
        .route("/admin/reels/{reel_id}/variants", post(variants))
        .route("/admin/reels/{reel_id}/submit-review", post(submit_review))
        .route("/admin/reels/{reel_id}/publish", post(publish))
        .route("/admin/reels/{reel_id}/archive", post(archive))
}

/// Reviewers (SME, manager) need to read a reel and record their decision.
pub fn reviewer_routes() -> Router<AppState> {
    Router::new()
        .route("/admin/reels/{reel_id}", get(get_reel))
        .route("/admin/reels/{reel_id}/approvals", post(approve))
}

#[derive(Deserialize)]
struct ListQuery {
    status: Option<ReelStatus>,
    topic_id: Option<Uuid>,
    language: Option<String>,
    kind: Option<ReelKind>,
    q: Option<String>,
    #[serde(flatten)]
    page: PageQuery,
}

async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Page<ReelListItem>>, ApiError> {
    let (cursor, limit) = page_params(&q.page)?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = reels::list(
        tx.conn(),
        &reels::ReelFilter {
            status: q.status,
            topic_id: q.topic_id,
            language: q.language,
            kind: q.kind,
            q: q.q,
        },
        cursor,
        limit,
    )
    .await?;
    let (rows, next) = page(rows, limit, |r| Cursor {
        ts: r.updated_at,
        id: r.id,
    });
    let topic_rows = topics::list(tx.conn()).await?;
    let mut items = Vec::with_capacity(rows.len());
    for r in &rows {
        let name = topic_rows
            .iter()
            .find(|t| t.id == r.topic_id)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        items.push(views::reel_list_item(&state, &mut tx, r, &name).await?);
    }
    tx.commit().await?;
    Ok(Json(Page {
        items,
        next_cursor: next,
    }))
}

async fn get_reel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reel_id): Path<Uuid>,
) -> Result<Json<Reel>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let r = reels::get(tx.conn(), reel_id).await?;
    let view = views::reel_admin(&state, &mut tx, &r).await?;
    tx.commit().await?;
    Ok(Json(view))
}

fn validate_question(
    q: &QuestionWrite,
    existing: Option<&reels::QuestionRow>,
) -> Result<(String, serde_json::Value, String, Option<String>), ApiError> {
    let prompt = q
        .prompt
        .clone()
        .or_else(|| existing.map(|e| e.prompt.clone()))
        .ok_or_else(|| ApiError::field("question.prompt", "is required"))?;
    let options = match &q.options {
        Some(o) => o.clone(),
        None => existing
            .map(|e| parse_options(&e.options))
            .unwrap_or_default(),
    };
    if !(2..=5).contains(&options.len()) {
        return Err(ApiError::field(
            "question.options",
            "must have 2 to 5 options",
        ));
    }
    let mut keys: Vec<&str> = options.iter().map(|o| o.key.as_str()).collect();
    keys.sort_unstable();
    keys.dedup();
    if keys.len() != options.len()
        || options
            .iter()
            .any(|o| o.key.trim().is_empty() || o.text.trim().is_empty())
    {
        return Err(ApiError::field(
            "question.options",
            "keys must be unique and texts non-empty",
        ));
    }
    let correct = q
        .correct_key
        .clone()
        .or_else(|| existing.map(|e| e.correct_key.clone()))
        .ok_or_else(|| ApiError::field("question.correct_key", "is required"))?;
    if !options.iter().any(|o| o.key == correct) {
        return Err(ApiError::field(
            "question.correct_key",
            "must be one of the option keys",
        ));
    }
    let explanation = q
        .explanation
        .clone()
        .or_else(|| existing.and_then(|e| e.explanation.clone()));
    Ok((prompt, serde_json::to_value(options)?, correct, explanation))
}

async fn invalidate_reel(state: &AppState, reel_id: Uuid) {
    state.cache.del_prefix(&keys::reel_prefix(reel_id)).await;
}

async fn patch(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reel_id): Path<Uuid>,
    Json(body): Json<ReelPatch>,
) -> Result<Json<Reel>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let mut r = reels::get_for_update(tx.conn(), reel_id).await?;
    if r.status == ReelStatus::Archived {
        return Err(ApiError::Conflict("archived reels cannot be edited".into()));
    }
    let content_changed = body.title.is_some()
        || body.script.is_some()
        || body.question.is_some()
        || body.presenter.is_some();
    if r.status == ReelStatus::Published && content_changed {
        // A new version goes back through review; learners who saw v1 keep their memory states.
        r = reels::bump_version_for_review(tx.conn(), r.id).await?;
    } else if matches!(r.status, ReelStatus::Approved | ReelStatus::Rejected) && content_changed {
        r = reels::set_status(tx.conn(), r.id, ReelStatus::Draft).await?;
    }
    let r = reels::update(
        tx.conn(),
        r.id,
        reels::ReelUpdate {
            title: body.title,
            script: body.script,
            source_citation: body.source_citation,
            presenter: body.presenter,
            compliance_tag: body.compliance_tag,
            ..Default::default()
        },
    )
    .await?;
    if let Some(q) = &body.question {
        let existing = reels::question_for_reel_in(tx.conn(), r.id, &r.language, &r.language)
            .await?
            .filter(|e| e.language == r.language);
        let (prompt, options, correct, explanation) = validate_question(q, existing.as_ref())?;
        reels::upsert_question(
            tx.conn(),
            auth.tenant_id,
            r.id,
            &r.language,
            &prompt,
            &options,
            &correct,
            explanation.as_deref(),
        )
        .await?;
    }
    record_audit(
        &mut tx,
        auth.user_id,
        "reel.update",
        "reel",
        Some(r.id),
        Some(serde_json::json!({ "version": r.version, "status": r.status })),
    )
    .await?;
    let view = views::reel_admin(&state, &mut tx, &r).await?;
    tx.commit().await?;
    invalidate_reel(&state, r.id).await;
    Ok(Json(view))
}

async fn start_reel_job(
    tx: &mut TenantTx,
    auth: &AuthUser,
    r: &reels::ReelRow,
    params: serde_json::Value,
    steps: &[GenStep],
    cost: i32,
) -> Result<GenerationJob, ApiError> {
    let source_id = r.source_id.ok_or_else(|| {
        ApiError::UnprocessableEntity("this reel has no source to regenerate from".into())
    })?;
    tenants::spend_credits(tx.conn(), auth.tenant_id, cost).await?;
    let job = studio::create_job(
        tx.conn(),
        auth.tenant_id,
        source_id,
        auth.user_id,
        None,
        &params,
        cost,
    )
    .await?;
    studio::insert_steps(tx.conn(), auth.tenant_id, job.id, steps).await?;
    let first = steps
        .first()
        .ok_or_else(|| ApiError::field("what", "nothing to regenerate"))?;
    jobs::enqueue(tx.conn(), Some(auth.tenant_id), first.job_kind(), serde_json::json!({ "generation_job_id": job.id, "position": 0, "source_id": source_id, "reel_id": r.id }), None).await?;
    job_view(tx, &job).await
}

async fn regenerate(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reel_id): Path<Uuid>,
    body: Option<Json<RegenerateRequest>>,
) -> Result<(StatusCode, Json<GenerationJob>), ApiError> {
    let what = body
        .and_then(|Json(b)| b.what)
        .unwrap_or_else(|| vec!["visual".into(), "voice".into()]);
    let allowed = ["visual", "voice", "script", "question"];
    if what.is_empty() || what.iter().any(|w| !allowed.contains(&w.as_str())) {
        return Err(ApiError::field(
            "what",
            "must contain visual, voice, script and/or question",
        ));
    }
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let r = reels::get(tx.conn(), reel_id).await?;
    let style = match r.kind {
        ReelKind::Motion => GenerationStyle::Motion,
        ReelKind::Broll => GenerationStyle::Broll,
        _ => GenerationStyle::AiAvatar,
    };
    let variant_count =
        i32::try_from(reels::variants(tx.conn(), r.id).await?.len()).unwrap_or(0) + 1;
    let q = pricing::quote_regenerate(&what, style, variant_count);
    let mut steps = Vec::new();
    for s in [
        GenStep::Script,
        GenStep::Question,
        GenStep::Render,
        GenStep::Voice,
        GenStep::Caption,
    ] {
        let wanted = match s {
            GenStep::Script => what.iter().any(|w| w == "script"),
            GenStep::Question => what.iter().any(|w| w == "question"),
            GenStep::Render => what.iter().any(|w| w == "visual"),
            GenStep::Voice | GenStep::Caption => what.iter().any(|w| w == "voice"),
            _ => false,
        };
        if wanted {
            steps.push(s);
        }
    }
    let params = serde_json::json!({ "kind": "regenerate", "reel_id": r.id, "what": what, "topic_id": r.topic_id, "languages": [r.language], "style": style, "reel_count": 1, "quote": q });
    let view = start_reel_job(&mut tx, &auth, &r, params, &steps, q.cost_credits).await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "reel.regenerate",
        "reel",
        Some(r.id),
        Some(serde_json::json!({ "what": what, "cost_credits": q.cost_credits })),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(view)))
}

async fn variants(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reel_id): Path<Uuid>,
    Json(body): Json<VariantsRequest>,
) -> Result<(StatusCode, Json<GenerationJob>), ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    let r = reels::get(tx.conn(), reel_id).await?;
    let existing: Vec<String> = reels::variants(tx.conn(), r.id)
        .await?
        .into_iter()
        .map(|v| v.language)
        .collect();
    let mut wanted: Vec<String> = Vec::new();
    for l in &body.languages {
        if !tenant.languages.contains(l) {
            return Err(ApiError::field(
                "languages",
                &format!("{l} is not one of the workspace languages"),
            ));
        }
        if l != &r.language && !existing.contains(l) && !wanted.contains(l) {
            wanted.push(l.clone());
        }
    }
    if wanted.is_empty() {
        return Err(ApiError::field(
            "languages",
            "every requested language already exists for this reel",
        ));
    }
    let q = pricing::quote_variants(i32::try_from(wanted.len()).unwrap_or(0));
    let params = serde_json::json!({ "kind": "variants", "reel_id": r.id, "variants": wanted, "topic_id": r.topic_id, "languages": wanted, "reel_count": 1, "quote": q });
    let view = start_reel_job(
        &mut tx,
        &auth,
        &r,
        params,
        &[GenStep::Translate],
        q.cost_credits,
    )
    .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "reel.variants",
        "reel",
        Some(r.id),
        Some(serde_json::json!({ "languages": wanted })),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(view)))
}

async fn submit_review(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reel_id): Path<Uuid>,
    body: Option<Json<SubmitReviewRequest>>,
) -> Result<Json<Reel>, ApiError> {
    let body = body.map(|Json(b)| b).unwrap_or_default();
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let r = reels::get_for_update(tx.conn(), reel_id).await?;
    if !matches!(
        r.status,
        ReelStatus::Draft | ReelStatus::Rejected | ReelStatus::InReview
    ) {
        return Err(ApiError::Conflict(format!(
            "a {} reel cannot be submitted for review",
            r.status
        )));
    }
    if reels::question_for_reel_in(tx.conn(), r.id, &r.language, &r.language)
        .await?
        .is_none()
    {
        return Err(ApiError::Conflict(
            "the reel needs a recall question before review".into(),
        ));
    }
    let r = reels::set_status(tx.conn(), r.id, ReelStatus::InReview).await?;
    if let Some(sme) = body.sme_user_id {
        let reviewer = users::get(tx.conn(), sme).await?;
        if !reviewer.role.at_least(reelearn_domain::Role::Sme) {
            return Err(ApiError::field(
                "sme_user_id",
                "must be an SME, manager or admin",
            ));
        }
        jobs::enqueue(
            tx.conn(),
            Some(auth.tenant_id),
            "send_delivery",
            serde_json::json!({ "kind": "review_request", "user_id": sme, "reel_id": r.id }),
            None,
        )
        .await?;
    }
    record_audit(
        &mut tx,
        auth.user_id,
        "reel.submit_review",
        "reel",
        Some(r.id),
        Some(serde_json::json!({ "sme_user_id": body.sme_user_id })),
    )
    .await?;
    let view = views::reel_admin(&state, &mut tx, &r).await?;
    tx.commit().await?;
    Ok(Json(view))
}

async fn approve(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reel_id): Path<Uuid>,
    Json(body): Json<ApprovalRequest>,
) -> Result<Json<Reel>, ApiError> {
    let role = auth
        .role
        .approval_role()
        .ok_or_else(|| ApiError::Forbidden("learners cannot sign off reels".into()))?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let r = reels::get_for_update(tx.conn(), reel_id).await?;
    if !matches!(r.status, ReelStatus::InReview | ReelStatus::Approved) {
        return Err(ApiError::Conflict(format!(
            "a {} reel is not awaiting sign-off",
            r.status
        )));
    }
    reels::add_approval(
        tx.conn(),
        auth.tenant_id,
        r.id,
        auth.user_id,
        role,
        body.decision,
        body.comment.as_deref(),
        r.version,
    )
    .await?;
    let next_status = match body.decision {
        Decision::Approved => {
            let roles = reels::approved_roles_for_version(tx.conn(), r.id, r.version).await?;
            if roles.contains(&ApprovalRole::Ld) && roles.contains(&ApprovalRole::Sme) {
                ReelStatus::Approved
            } else {
                ReelStatus::InReview
            }
        }
        Decision::ChangesRequested => ReelStatus::Draft,
        Decision::Rejected => ReelStatus::Rejected,
    };
    let r = if next_status != r.status {
        reels::set_status(tx.conn(), r.id, next_status).await?
    } else {
        r
    };
    record_audit(
        &mut tx,
        auth.user_id,
        "reel.approval",
        "reel",
        Some(r.id),
        Some(serde_json::json!({ "role": role, "decision": body.decision, "version": r.version })),
    )
    .await?;
    let view = views::reel_admin(&state, &mut tx, &r).await?;
    tx.commit().await?;
    Ok(Json(view))
}

async fn publish(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reel_id): Path<Uuid>,
    Json(body): Json<PublishRequest>,
) -> Result<Json<Reel>, ApiError> {
    if body.group_ids.is_empty() {
        return Err(ApiError::field(
            "group_ids",
            "at least one audience group is required",
        ));
    }
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let r = reels::get_for_update(tx.conn(), reel_id).await?;
    if matches!(r.status, ReelStatus::Archived | ReelStatus::Rejected) {
        return Err(ApiError::Conflict(format!(
            "a {} reel cannot be published",
            r.status
        )));
    }
    if !groups::ids_exist(tx.conn(), &body.group_ids).await? {
        return Err(ApiError::field("group_ids", "contains an unknown group"));
    }
    if r.video_key.is_none() {
        return Err(ApiError::Conflict("the reel has no video yet".into()));
    }
    let roles = reels::approved_roles_for_version(tx.conn(), r.id, r.version).await?;
    if !(roles.contains(&ApprovalRole::Ld) && roles.contains(&ApprovalRole::Sme)) {
        return Err(ApiError::Conflict(format!(
            "version {} needs both an L&D and an SME approval before publishing",
            r.version
        )));
    }
    if r.kind == ReelKind::SmeRecorded && !r.consent_confirmed {
        return Err(ApiError::Conflict(
            "speaker consent is required before publishing a recording".into(),
        ));
    }
    if let Some(tag) = body.compliance_tag.as_deref() {
        reels::update(
            tx.conn(),
            r.id,
            reels::ReelUpdate {
                compliance_tag: Some(tag.to_string()),
                ..Default::default()
            },
        )
        .await?;
    }
    reels::set_audience(
        tx.conn(),
        auth.tenant_id,
        r.id,
        &body.group_ids,
        body.starts_on,
    )
    .await?;
    // The trigger `reels_require_dual_approval` re-checks the sign-offs; a failure maps to 409.
    let r = reels::publish(tx.conn(), r.id).await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "reel.publish",
        "reel",
        Some(r.id),
        Some(serde_json::json!({ "groups": body.group_ids, "version": r.version })),
    )
    .await?;
    let view = views::reel_admin(&state, &mut tx, &r).await?;
    tx.commit().await?;
    invalidate_reel(&state, r.id).await;
    Ok(Json(view))
}

async fn archive(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(reel_id): Path<Uuid>,
) -> Result<Json<Reel>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    let r = reels::get_for_update(tx.conn(), reel_id).await?;
    let r = reels::set_status(tx.conn(), r.id, ReelStatus::Archived).await?;
    let dropped = plans::drop_pending_items_for_reel(
        tx.conn(),
        r.id,
        local_today(&tenant.timezone, Utc::now()),
    )
    .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "reel.archive",
        "reel",
        Some(r.id),
        Some(serde_json::json!({ "dropped_plan_items": dropped })),
    )
    .await?;
    let view = views::reel_admin(&state, &mut tx, &r).await?;
    tx.commit().await?;
    invalidate_reel(&state, r.id).await;
    Ok(Json(view))
}
