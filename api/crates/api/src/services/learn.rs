//! The learner hot path: today's plan (cached), watch events, answers with SM-2
//! scheduling, stats and cache invalidation. Shared by `/me/*` and the WhatsApp bot.

use crate::cache::keys;
use crate::dto::*;
use crate::error::ApiError;
use crate::services::{stats as stats_svc, views};
use crate::state::AppState;
use chrono::{NaiveDate, Utc};
use reelearn_db::repos::{
    learning, plans as plan_repo, reels, stats, tenants, tenants::TenantRow, topics, users,
    users::UserRow,
};
use reelearn_db::settings::{local_now, local_today, TenantSettings};
use reelearn_db::{plans, TenantTx};
use reelearn_domain::recall::{self, MemoryState};
use reelearn_domain::{Channel, PlanItemKind, PlanItemReason, PlanItemState};
use std::time::Duration;
use uuid::Uuid;

pub struct LearnerCtx {
    pub tenant: TenantRow,
    pub settings: TenantSettings,
    pub user: UserRow,
    pub today: NaiveDate,
}

pub async fn load_ctx(tx: &mut TenantTx, user_id: Uuid) -> Result<LearnerCtx, ApiError> {
    let tenant_id = tx.tenant_id();
    let tenant = tenants::get(tx.conn(), tenant_id).await?;
    let user = users::get(tx.conn(), user_id).await?;
    let settings = TenantSettings::from_json(&tenant.settings);
    let today = local_today(&tenant.timezone, Utc::now());
    Ok(LearnerCtx {
        tenant,
        settings,
        user,
        today,
    })
}

fn seconds_until_local_midnight(timezone: &str) -> Duration {
    let now = local_now(timezone, Utc::now());
    let tomorrow = now.date().succ_opt().unwrap_or(now.date());
    let midnight = tomorrow.and_hms_opt(0, 0, 0).unwrap_or(now);
    let secs = (midnight - now).num_seconds().clamp(60, 24 * 3600);
    Duration::from_secs(u64::try_from(secs).unwrap_or(3600))
}

/// Today's plan: Redis hit, else materialise and cache until the learner's day ends.
pub async fn today(
    state: &AppState,
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
) -> Result<DailyPlan, ApiError> {
    let cache_key = keys::plan(ctx.tenant.id, ctx.user.id, ctx.today);
    if let Some(plan) = state.cache.get_json::<DailyPlan>(&cache_key).await {
        return Ok(plan);
    }
    let plan = build_today(state, tx, ctx).await?;
    state
        .cache
        .set_json(
            &cache_key,
            &plan,
            seconds_until_local_midnight(&ctx.tenant.timezone),
        )
        .await;
    Ok(plan)
}

pub async fn build_today(
    state: &AppState,
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
) -> Result<DailyPlan, ApiError> {
    let tenant_id = tx.tenant_id();
    let materialized =
        plans::ensure_plan(tx.conn(), tenant_id, &ctx.settings, &ctx.user, ctx.today).await?;
    let mut items = Vec::with_capacity(materialized.items.len());
    for item in &materialized.items {
        items.push(plan_item_view(state, tx, ctx, item).await?);
    }
    let stats_row = stats::user_stats(tx.conn(), ctx.user.id).await?;
    let yesterday = match ctx.today.pred_opt() {
        Some(y) => match plan_repo::find_for_date(tx.conn(), ctx.user.id, y).await? {
            Some(p) if p.item_count > 0 => Some(Yesterday {
                remembered: learning::plan_correct_count(tx.conn(), p.id).await?,
                total: i64::from(p.item_count),
                next_review_on: learning::next_due_on(tx.conn(), ctx.user.id, ctx.today).await?,
            }),
            _ => None,
        },
        None => None,
    };
    let status = if materialized.plan.item_count == 0 {
        "empty".to_string()
    } else {
        materialized.plan.status.clone()
    };
    let estimated_seconds = items
        .iter()
        .map(|i| if i.kind == PlanItemKind::Reel { 30 } else { 10 })
        .sum();
    Ok(DailyPlan {
        id: materialized.plan.id,
        plan_date: materialized.plan.plan_date,
        status,
        greeting_name: ctx.user.first_name(),
        estimated_seconds,
        items,
        stats: views::user_stats(stats_row.as_ref()),
        yesterday,
        practice_available: crate::services::engine::practice_available(ctx),
    })
}

async fn plan_item_view(
    state: &AppState,
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    item: &plan_repo::PlanItemRow,
) -> Result<DailyPlanItem, ApiError> {
    let lang = ctx.user.preferred_language.as_str();
    let reel_row = match item.reel_id {
        Some(id) => reels::find(tx.conn(), id).await?,
        None => None,
    };
    let topic_name = match &reel_row {
        Some(r) => topics::get(tx.conn(), r.topic_id)
            .await
            .map(|t| t.name)
            .unwrap_or_default(),
        None => String::new(),
    };
    let reel = match (&reel_row, item.kind) {
        (Some(r), PlanItemKind::Reel) => {
            Some(views::reel_playable(state, tx, &ctx.user, r, lang, &topic_name).await?)
        }
        _ => None,
    };
    let question = match item.question_id {
        Some(qid) => reels::question(tx.conn(), qid)
            .await?
            .map(|q| views::question(&q, false)),
        None => None,
    };
    let review_note = if item.reason == PlanItemReason::Review {
        match item.question_id {
            Some(qid) => learning::memory_state(tx.conn(), ctx.user.id, qid)
                .await?
                .map(|m| recall::review_note(&MemoryState::from(&m))),
            None => None,
        }
    } else {
        None
    };
    Ok(DailyPlanItem {
        id: item.id,
        position: item.position,
        kind: item.kind,
        reason: item.reason,
        state: item.state,
        reel,
        question,
        review_note,
    })
}

pub async fn invalidate_plan_cache(state: &AppState, tenant_id: Uuid, user_id: Uuid) {
    state
        .cache
        .del_prefix(&keys::plan_prefix(tenant_id, user_id))
        .await;
}

/// Record a play; counts a completion once per plan item (or once a day without one).
pub async fn record_watch(
    state: &AppState,
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    req: &WatchRequest,
    device_id: Option<&str>,
) -> Result<(), ApiError> {
    let tenant_id = tx.tenant_id();
    let reel = reels::find(tx.conn(), req.reel_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("reel not found".into()))?;
    let item = match req.plan_item_id {
        Some(id) => {
            let item = plan_repo::item(tx.conn(), id)
                .await?
                .ok_or_else(|| ApiError::NotFound("plan item not found".into()))?;
            let plan = plan_repo::get(tx.conn(), item.plan_id).await?;
            if plan.user_id != ctx.user.id {
                return Err(ApiError::NotFound("plan item not found".into()));
            }
            Some(item)
        }
        None => match plan_repo::find_for_date(tx.conn(), ctx.user.id, ctx.today).await? {
            Some(plan) => plan_repo::item_for_reel(tx.conn(), plan.id, reel.id).await?,
            None => None,
        },
    };
    let channel = req.channel.unwrap_or(Channel::App);
    learning::insert_watch_event(
        tx.conn(),
        tenant_id,
        ctx.user.id,
        reel.id,
        item.as_ref().map(|i| i.id),
        channel,
        device_id,
        req.watched_ms.max(0),
        req.completed,
    )
    .await?;
    crate::services::engine::on_watch(
        tx,
        ctx,
        &reel,
        req.watched_ms.max(0),
        req.completed,
        channel,
    )
    .await?;

    let counts = if req.completed {
        match &item {
            Some(i) if i.state == PlanItemState::Pending => {
                plan_repo::set_item_state(tx.conn(), i.id, PlanItemState::Watched).await?;
                true
            }
            Some(_) => false,
            None => !learning::has_completed_watch_today(tx.conn(), ctx.user.id, reel.id).await?,
        }
    } else {
        false
    };
    if counts {
        let topic = topics::get(tx.conn(), reel.topic_id).await?;
        let minutes = f64::from(reel.duration_ms.unwrap_or(30_000).max(0)) / 60_000.0;
        let before = stats::user_stats(tx.conn(), ctx.user.id).await?;
        let first_today = before.as_ref().and_then(|s| s.last_active_on) != Some(ctx.today);
        stats_svc::record_activity(
            tx,
            &ctx.settings,
            &ctx.user,
            ctx.today,
            stats_svc::ActivityDelta {
                reels_completed: 1,
                answers_total: 0,
                answers_correct: 0,
                minutes_on_mandatory: if topic.is_mandatory { minutes } else { 0.0 },
            },
        )
        .await?;
        stats_svc::bump_daily_topic(
            tx,
            ctx.user.id,
            reel.topic_id,
            ctx.today,
            first_today,
            1,
            0,
            0,
            minutes,
        )
        .await?;
        stats_svc::refresh_readiness(tx, ctx.user.id, reel.topic_id, ctx.today).await?;
    }
    users::touch_last_seen(tx.conn(), ctx.user.id).await?;
    invalidate_plan_cache(state, tenant_id, ctx.user.id).await;
    Ok(())
}

/// Answer a recall question: SM-2 reschedule, plan progress, stats, readiness.
pub async fn record_answer(
    state: &AppState,
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    req: &AnswerRequest,
) -> Result<AnswerOutcome, ApiError> {
    let tenant_id = tx.tenant_id();
    let question = reels::question(tx.conn(), req.question_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("question not found".into()))?;
    let reel = reels::get(tx.conn(), question.reel_id).await?;
    let options = parse_options(&question.options);
    if !options.iter().any(|o| o.key == req.chosen_key) {
        return Err(ApiError::field(
            "chosen_key",
            "is not one of the question's options",
        ));
    }

    let item = match req.plan_item_id {
        Some(id) => {
            let item = plan_repo::item(tx.conn(), id)
                .await?
                .ok_or_else(|| ApiError::NotFound("plan item not found".into()))?;
            let plan = plan_repo::get(tx.conn(), item.plan_id).await?;
            if plan.user_id != ctx.user.id {
                return Err(ApiError::NotFound("plan item not found".into()));
            }
            Some(item)
        }
        None => match plan_repo::find_for_date(tx.conn(), ctx.user.id, ctx.today).await? {
            Some(plan) => plan_repo::item_for_question(tx.conn(), plan.id, question.id).await?,
            None => None,
        },
    };
    if let Some(i) = &item {
        if i.state == PlanItemState::Answered {
            return Err(ApiError::Conflict(
                "this question was already answered in today's plan".into(),
            ));
        }
    }

    let is_correct = req.chosen_key == question.correct_key;
    let now = Utc::now();
    let previous = learning::memory_state(tx.conn(), ctx.user.id, question.id).await?;
    let prev_state = previous
        .as_ref()
        .map(MemoryState::from)
        .unwrap_or_else(|| MemoryState::fresh(ctx.today));
    let attempt_no = previous
        .as_ref()
        .map(|p| p.reps.saturating_add(p.lapses).saturating_add(1))
        .unwrap_or(1);
    let next = recall::review(&prev_state, is_correct, ctx.today).state;
    learning::upsert_memory_state(
        tx.conn(),
        tenant_id,
        ctx.user.id,
        question.id,
        &next,
        Some(now),
    )
    .await?;
    learning::insert_answer(
        tx.conn(),
        tenant_id,
        ctx.user.id,
        question.id,
        reel.id,
        item.as_ref().map(|i| i.id),
        &req.chosen_key,
        is_correct,
        req.response_ms,
        attempt_no,
        req.channel.unwrap_or(Channel::App),
        now,
    )
    .await?;

    crate::services::engine::on_answer(tx, ctx, &reel, is_correct).await?;

    let plan_status = match &item {
        Some(i) => {
            plan_repo::set_item_state(tx.conn(), i.id, PlanItemState::Answered).await?;
            plan_repo::refresh_progress(tx.conn(), i.plan_id)
                .await?
                .status
        }
        None => plan_repo::find_for_date(tx.conn(), ctx.user.id, ctx.today)
            .await?
            .map(|p| p.status)
            .unwrap_or_else(|| "open".to_string()),
    };

    let before = stats::user_stats(tx.conn(), ctx.user.id).await?;
    let first_today = before.as_ref().and_then(|s| s.last_active_on) != Some(ctx.today);
    let stats_row = stats_svc::record_activity(
        tx,
        &ctx.settings,
        &ctx.user,
        ctx.today,
        stats_svc::ActivityDelta {
            reels_completed: 0,
            answers_total: 1,
            answers_correct: i32::from(is_correct),
            minutes_on_mandatory: 0.0,
        },
    )
    .await?;
    stats_svc::bump_daily_topic(
        tx,
        ctx.user.id,
        reel.topic_id,
        ctx.today,
        first_today,
        0,
        1,
        i32::from(is_correct),
        0.0,
    )
    .await?;
    stats_svc::refresh_readiness(tx, ctx.user.id, reel.topic_id, ctx.today).await?;
    users::touch_last_seen(tx.conn(), ctx.user.id).await?;
    invalidate_plan_cache(state, tenant_id, ctx.user.id).await;

    Ok(AnswerOutcome {
        is_correct,
        correct_key: question.correct_key.clone(),
        explanation: question.explanation.clone(),
        next_due_on: next.due_on,
        plan_status,
        stats: views::user_stats(Some(&stats_row)),
    })
}

pub async fn progress(tx: &mut TenantTx, ctx: &LearnerCtx) -> Result<Progress, ApiError> {
    let stats_row = stats::user_stats(tx.conn(), ctx.user.id).await?;
    let total = learning::memory_count(tx.conn(), ctx.user.id).await?;
    let remembered = learning::remembered_count(tx.conn(), ctx.user.id).await?;
    let remembered_pct = if total > 0 {
        i32::try_from(100 * remembered / total).unwrap_or(0)
    } else {
        0
    };
    let series = learning::weekly_recall(tx.conn(), ctx.user.id, 8)
        .await?
        .into_iter()
        .map(|w| ProgressWeek {
            week: w.week,
            remembered_pct: if w.total > 0 {
                i32::try_from(100 * w.correct / w.total).unwrap_or(0)
            } else {
                0
            },
        })
        .collect();
    let mut by_topic = Vec::new();
    let memory = learning::memory_by_topic(tx.conn(), ctx.user.id, ctx.today).await?;
    let readiness = stats::readiness_for_user(tx.conn(), ctx.user.id).await?;
    for t in topics::list(tx.conn()).await? {
        let m = memory.iter().find(|m| m.topic_id == t.id);
        let r = readiness.iter().find(|r| r.topic_id == t.id);
        if m.is_none() && r.is_none() {
            continue;
        }
        let recall_pct = r
            .map(|r| i32::from(r.recall_pct))
            .or_else(|| {
                m.filter(|m| m.total > 0)
                    .map(|m| i32::try_from(100 * m.remembered / m.total).unwrap_or(0))
            })
            .unwrap_or(0);
        by_topic.push(ProgressTopic {
            topic_id: t.id,
            name: t.name.clone(),
            recall_pct,
            next_review_on: m.and_then(|m| m.next_due_on),
        });
    }
    let s = views::user_stats(stats_row.as_ref());
    let milestones = vec![
        Milestone {
            key: "first_reel".into(),
            label: "First reel".into(),
            reached: s.reels_completed >= 1,
        },
        Milestone {
            key: "streak_7".into(),
            label: "7-day streak".into(),
            reached: s.streak_longest >= 7,
        },
        Milestone {
            key: "remembered_10".into(),
            label: "10 things remembered".into(),
            reached: s.remembered_count >= 10,
        },
        Milestone {
            key: "topic_ready".into(),
            label: "A topic ready".into(),
            reached: readiness.iter().any(|r| r.ready),
        },
    ];
    Ok(Progress {
        remembered_pct,
        series,
        by_topic,
        milestones,
        stats: s,
    })
}
