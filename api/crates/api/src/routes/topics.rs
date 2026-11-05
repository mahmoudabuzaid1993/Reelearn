//! `/admin/topics`: topics and the assignment matrix (R01).

use crate::auth::AuthUser;
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::{record_audit, Json, Path};
use crate::services::views;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use reelearn_db::repos::{groups, topics};
use reelearn_domain::Channel;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/topics", get(list).post(create))
        .route("/admin/topics/{topic_id}", axum::routing::patch(patch))
        .route(
            "/admin/topics/{topic_id}/assignments",
            get(assignments).put(replace_assignments),
        )
}

pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if (c.is_whitespace() || c == '-' || c == '_') && !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        } else if c.is_alphanumeric() {
            out.push(c);
            last_dash = false;
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        format!("topic-{}", &Uuid::now_v7().to_string()[..8])
    } else {
        out
    }
}

async fn list(State(state): State<AppState>, auth: AuthUser) -> Result<Json<Vec<Topic>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = topics::list(tx.conn()).await?;
    tx.commit().await?;
    Ok(Json(rows.iter().map(views::topic).collect()))
}

async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<TopicCreate>,
) -> Result<(StatusCode, Json<Topic>), ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::field("name", "is required"));
    }
    let slug = body
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&body.name));
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let t = topics::create(
        tx.conn(),
        auth.tenant_id,
        topics::TopicWrite {
            slug: &slug,
            name: body.name.trim(),
            name_ar: body.name_ar.as_deref(),
            description: body.description.as_deref(),
            compliance_tag: body.compliance_tag.as_deref(),
            is_mandatory: body.is_mandatory.unwrap_or(false),
        },
    )
    .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "topic.create",
        "topic",
        Some(t.id),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(views::topic(&t))))
}

async fn patch(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(topic_id): Path<Uuid>,
    Json(body): Json<TopicCreate>,
) -> Result<Json<Topic>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let t = topics::update(
        tx.conn(),
        topic_id,
        topics::TopicPatch {
            slug: body.slug.as_deref().map(slugify),
            name: Some(body.name.trim().to_string()).filter(|n| !n.is_empty()),
            name_ar: body.name_ar,
            description: body.description,
            compliance_tag: body.compliance_tag,
            is_mandatory: body.is_mandatory,
            status: None,
        },
    )
    .await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "topic.update",
        "topic",
        Some(t.id),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(views::topic(&t)))
}

async fn assignments(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(topic_id): Path<Uuid>,
) -> Result<Json<Vec<Assignment>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    topics::get(tx.conn(), topic_id).await?;
    let rows = topics::assignments_for_topic(tx.conn(), topic_id).await?;
    tx.commit().await?;
    Ok(Json(rows.iter().map(views::assignment).collect()))
}

async fn replace_assignments(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(topic_id): Path<Uuid>,
    Json(body): Json<Vec<AssignmentWrite>>,
) -> Result<Json<Vec<Assignment>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    topics::get(tx.conn(), topic_id).await?;
    let ids: Vec<_> = body.iter().map(|a| a.group_id).collect();
    if !groups::ids_exist(tx.conn(), &ids).await? {
        return Err(ApiError::field("group_id", "contains an unknown group"));
    }
    let mut writes = Vec::with_capacity(body.len());
    for a in &body {
        let per_day = a.reels_per_day.unwrap_or(2);
        if !(0..=5).contains(&per_day) {
            return Err(ApiError::field("reels_per_day", "must be between 0 and 5"));
        }
        writes.push(topics::AssignmentWrite {
            group_id: a.group_id,
            reels_per_day: per_day,
            channel: a.channel.unwrap_or(Channel::App),
            is_mandatory: a.is_mandatory.unwrap_or(false),
            starts_on: a.starts_on,
            ends_on: a.ends_on,
        });
    }
    let rows = topics::replace_assignments(tx.conn(), auth.tenant_id, topic_id, &writes).await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "topic.assignments",
        "topic",
        Some(topic_id),
        Some(serde_json::json!({ "groups": ids })),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(rows.iter().map(views::assignment).collect()))
}
