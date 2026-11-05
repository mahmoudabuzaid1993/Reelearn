//! `/manager/*`: readiness per person per topic, nudges, anonymised reel requests.
//! Reads only `user_topic_readiness`, `user_stats` and `deliveries`: no question ids,
//! chosen answers, per-answer timestamps or saved reels ever leave this module.

use crate::auth::AuthUser;
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::Json;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use reelearn_db::repos::{deliveries, jobs, learning, stats, tenants, topics, users};
use reelearn_db::settings::local_today;
use reelearn_domain::Channel;
use std::collections::HashSet;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/manager/team", get(team))
        .route("/manager/nudges", post(nudges))
        .route("/manager/reel-requests", get(reel_requests))
}

async fn team(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<TeamReadiness>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    let today = local_today(&tenant.timezone, Utc::now());
    let members = users::team_of(tx.conn(), auth.user_id).await?;
    let ids: Vec<_> = members.iter().map(|m| m.id).collect();
    let readiness = stats::readiness_for_users(tx.conn(), &ids).await?;
    let last_active = stats::last_active_for_users(tx.conn(), &ids).await?;
    let nudged: HashSet<_> = deliveries::nudged_on(tx.conn(), &ids, today)
        .await?
        .into_iter()
        .collect();
    let all_topics = topics::list(tx.conn()).await?;

    // Topics any member is assigned to.
    let mut assigned: HashSet<uuid::Uuid> = HashSet::new();
    for m in &members {
        assigned.extend(topics::topic_ids_for_user(tx.conn(), m.id).await?);
    }
    let topic_rows: Vec<_> = all_topics
        .iter()
        .filter(|t| assigned.contains(&t.id))
        .collect();

    let week_start = today - chrono::Duration::days(6);
    let active_this_week = last_active
        .iter()
        .filter(|l| l.last_active_on.map(|d| d >= week_start).unwrap_or(false))
        .count();
    let (sum, n) = readiness.iter().fold((0i64, 0i64), |(s, n), r| {
        (s + i64::from(r.recall_pct), n + 1)
    });
    let team_recall_pct = if n > 0 {
        i32::try_from(sum / n).unwrap_or(0)
    } else {
        0
    };

    let topics_out = topic_rows
        .iter()
        .map(|t| TeamTopic {
            id: t.id,
            name: t.name.clone(),
            ready_count: readiness
                .iter()
                .filter(|r| r.topic_id == t.id && r.ready)
                .count() as i64,
        })
        .collect();

    let members_out = members
        .iter()
        .map(|m| {
            let my_topics: Vec<_> = topic_rows.iter().map(|t| t.id).collect();
            TeamMember {
                user_id: m.id,
                full_name: m.full_name.clone(),
                language: m.preferred_language.clone(),
                last_active_on: last_active
                    .iter()
                    .find(|l| l.user_id == m.id)
                    .and_then(|l| l.last_active_on),
                nudged_today: nudged.contains(&m.id),
                readiness: my_topics
                    .iter()
                    .map(|tid| {
                        let state = match readiness
                            .iter()
                            .find(|r| r.user_id == m.id && r.topic_id == *tid)
                        {
                            Some(r) if r.ready => "ready",
                            Some(r) if r.reels_seen > 0 => "not_yet",
                            _ => "not_started",
                        };
                        MemberReadiness {
                            topic_id: *tid,
                            state: state.to_string(),
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    tx.commit().await?;
    Ok(Json(TeamReadiness {
        team_recall_pct,
        active_this_week: active_this_week as i64,
        team_size: members.len() as i64,
        topics: topics_out,
        members: members_out,
    }))
}

async fn nudges(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<NudgeRequest>,
) -> Result<(StatusCode, Json<NudgeResponse>), ApiError> {
    if body.user_ids.len() > 200 {
        return Err(ApiError::field(
            "user_ids",
            "at most 200 people per request",
        ));
    }
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    let today = local_today(&tenant.timezone, Utc::now());
    let team: HashSet<_> = if auth.is_admin() {
        body.user_ids.iter().copied().collect()
    } else {
        users::team_of(tx.conn(), auth.user_id)
            .await?
            .into_iter()
            .map(|u| u.id)
            .collect()
    };
    let targets: Vec<_> = body
        .user_ids
        .iter()
        .copied()
        .filter(|id| team.contains(id))
        .collect();
    let already: HashSet<_> = deliveries::nudged_on(tx.conn(), &targets, today)
        .await?
        .into_iter()
        .collect();
    let mut queued = 0;
    for user_id in &targets {
        if already.contains(user_id) {
            continue;
        }
        let Some(user) = users::find(tx.conn(), *user_id).await? else {
            continue;
        };
        let channel = if user.delivery_channel.includes_whatsapp() {
            Channel::Whatsapp
        } else {
            Channel::App
        };
        let delivery = deliveries::schedule(
            tx.conn(),
            auth.tenant_id,
            *user_id,
            None,
            channel,
            "nudge",
            Utc::now(),
        )
        .await?;
        jobs::enqueue(tx.conn(), Some(auth.tenant_id), "send_delivery", serde_json::json!({ "delivery_id": delivery.id, "scheduled_for": delivery.scheduled_for, "kind": "nudge", "from": auth.user_id }), None).await?;
        queued += 1;
    }
    tx.commit().await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(NudgeResponse {
            queued,
            skipped_already_nudged: targets.len() - queued,
        }),
    ))
}

async fn reel_requests(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<learning::ReelRequestRow>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = if auth.is_admin() {
        learning::all_reel_requests(tx.conn()).await?
    } else {
        let ids: Vec<_> = users::team_of(tx.conn(), auth.user_id)
            .await?
            .into_iter()
            .map(|u| u.id)
            .collect();
        learning::reel_requests_for_users(tx.conn(), &ids).await?
    };
    tx.commit().await?;
    Ok(Json(rows))
}
