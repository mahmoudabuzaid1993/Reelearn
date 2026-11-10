//! The learning engine at request time: practice batches (`/me/next`), signals from the phone,
//! the person's topics, colleague sharing, and the company's tuning and insights.
//!
//! Ranking itself is pure (`reelearn_domain::engine`); loading is in `reelearn_db::engine`.

use crate::dto::*;
use crate::error::ApiError;
use crate::services::learn::LearnerCtx;
use crate::services::views;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use reelearn_db::engine as loader;
use reelearn_db::repos::engine::{self as repo, Scope};
use reelearn_db::repos::{reels, reels::ReelRow, topics};
use reelearn_db::TenantTx;
use reelearn_domain::engine::signals::{
    interest_delta, moves_interest, ANSWER_CORRECT_DELTA, ANSWER_WRONG_DELTA,
};
use reelearn_domain::engine::{self as eng, rank, EngineSettings, Preference, RankReason, Usage};
use reelearn_domain::recall::{self, MemoryState};
use reelearn_domain::{Channel, EngagementKind, PlanItemKind};
use uuid::Uuid;

/// Signals the phone may send through `/me/events`. Completions arrive with `/me/watch`,
/// saves and shares through their own endpoints, impressions are written by the ranker.
pub const CLIENT_KINDS: [EngagementKind; 5] = [
    EngagementKind::Start,
    EngagementKind::Skip,
    EngagementKind::Rewatch,
    EngagementKind::NotRelevant,
    EngagementKind::MoreLikeThis,
];
pub const MAX_EVENTS_PER_BATCH: usize = 50;
pub const MAX_ACTIVE_MS_PER_BATCH: i64 = 180_000;
pub const MAX_SHARES_PER_DAY: i64 = 30;
pub const MAX_RECIPIENTS: usize = 10;
/// Locations with fewer people than this are left out of insights, so no one is singled out.
pub const MIN_GROUP_FOR_REPORTS: i64 = 3;

pub fn settings(ctx: &LearnerCtx) -> &EngineSettings {
    &ctx.settings.engine
}

pub use reelearn_db::settings::local_day_start;

/// 0..100 for display: tanh of the positive part of the score.
pub fn interest_level(score: f32) -> i32 {
    (100.0 * (score.max(0.0) / 10.0).tanh()).round() as i32
}

fn budget_view(b: &eng::Budget) -> PracticeBudget {
    PracticeBudget {
        open: b.open,
        stop: b.stop.map(|s| s.as_str().to_string()),
        remaining_today_s: b.remaining_today_s,
        remaining_session_s: b.remaining_session_s,
        break_every_s: b.break_every_s,
    }
}

pub fn practice_available(ctx: &LearnerCtx) -> bool {
    settings(ctx).continuous_enabled && !ctx.user.is_paused_on(ctx.today)
}

// ---------------------------------------------------------------- signals

/// Record one signal: the event row, the reel's running counters, the person's interest in
/// the topic and tags, and (on a start) any colleague share of this reel as opened.
#[allow(clippy::too_many_arguments)]
pub async fn apply_signal(
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    reel: &ReelRow,
    kind: EngagementKind,
    watched_ms: Option<i32>,
    session_id: Option<Uuid>,
    position: Option<i16>,
    channel: Channel,
) -> Result<(), ApiError> {
    let tenant_id = tx.tenant_id();
    let hl = settings(ctx).interest_half_life_days;
    repo::insert_event(
        tx.conn(),
        tenant_id,
        ctx.user.id,
        reel.id,
        session_id,
        kind,
        watched_ms,
        position,
        None,
        None,
        channel,
    )
    .await?;
    repo::bump_reel_stats(
        tx.conn(),
        tenant_id,
        reel.id,
        kind,
        i64::from(watched_ms.unwrap_or(0)),
    )
    .await?;
    if moves_interest(kind) {
        let delta = interest_delta(kind);
        repo::add_topic_interest(tx.conn(), tenant_id, ctx.user.id, reel.topic_id, delta, hl)
            .await?;
        let tags = reel_tags(tx, reel.id).await?;
        repo::add_tag_interest(tx.conn(), tenant_id, ctx.user.id, &tags, delta * 0.5, hl).await?;
    }
    if kind == EngagementKind::Start {
        repo::mark_shares_opened(tx.conn(), ctx.user.id, reel.id).await?;
    }
    Ok(())
}

async fn reel_tags(tx: &mut TenantTx, reel_id: Uuid) -> Result<Vec<String>, ApiError> {
    Ok(
        sqlx::query_scalar::<_, Vec<String>>("SELECT tags FROM reels WHERE id = $1")
            .bind(reel_id)
            .fetch_optional(tx.conn())
            .await
            .map_err(reelearn_db::DbError::from)?
            .unwrap_or_default(),
    )
}

/// What a `/me/watch` call says about engagement: the arrival is a start, a sub-3-second
/// play a skip, a 90 % play a completion.
pub async fn on_watch(
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    reel: &ReelRow,
    watched_ms: i32,
    completed: bool,
    channel: Channel,
) -> Result<(), ApiError> {
    let kind = if completed {
        Some(EngagementKind::Complete)
    } else if watched_ms == 0 {
        Some(EngagementKind::Start)
    } else {
        eng::signals::classify_watch(watched_ms, reel.duration_ms.unwrap_or(30_000), false)
    };
    if let Some(kind) = kind {
        apply_signal(tx, ctx, reel, kind, Some(watched_ms), None, None, channel).await?;
    }
    Ok(())
}

/// An answer feeds the reel's difficulty estimate and nudges interest in its topic.
pub async fn on_answer(
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    reel: &ReelRow,
    correct: bool,
) -> Result<(), ApiError> {
    let tenant_id = tx.tenant_id();
    repo::bump_reel_answers(tx.conn(), tenant_id, reel.id, correct).await?;
    let delta = if correct {
        ANSWER_CORRECT_DELTA
    } else {
        ANSWER_WRONG_DELTA
    };
    repo::add_topic_interest(
        tx.conn(),
        tenant_id,
        ctx.user.id,
        reel.topic_id,
        delta,
        settings(ctx).interest_half_life_days,
    )
    .await?;
    Ok(())
}

pub async fn record_events(
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    batch: &EngagementBatch,
) -> Result<i32, ApiError> {
    if batch.events.len() > MAX_EVENTS_PER_BATCH {
        return Err(ApiError::field("events", "at most 50 events per request"));
    }
    if let Some(bad) = batch
        .events
        .iter()
        .find(|e| !CLIENT_KINDS.contains(&e.kind))
    {
        return Err(ApiError::field(
            "events.kind",
            &format!(
                "{} is recorded by the server, not sent by the app",
                bad.kind
            ),
        ));
    }
    let session_id = match batch.session_id {
        Some(id) => repo::session(tx.conn(), id)
            .await?
            .filter(|s| s.user_id == ctx.user.id)
            .map(|s| s.id),
        None => None,
    };
    let open_catalog = settings(ctx).open_catalog;
    let mut recorded = 0;
    for e in &batch.events {
        let Some(reel) = reels::find(tx.conn(), e.reel_id).await? else {
            continue;
        };
        if !repo::may_play(tx.conn(), ctx.user.id, reel.id, open_catalog).await? {
            continue;
        }
        apply_signal(
            tx,
            ctx,
            &reel,
            e.kind,
            e.watched_ms.map(|w| w.max(0)),
            session_id,
            e.position,
            Channel::App,
        )
        .await?;
        recorded += 1;
    }
    if let Some(id) = session_id {
        let active = batch
            .active_ms
            .unwrap_or(0)
            .clamp(0, MAX_ACTIVE_MS_PER_BATCH);
        repo::touch_session(tx.conn(), id, 0, 0, active).await?;
    }
    Ok(recorded)
}

// ---------------------------------------------------------------- practice batches

fn first_name(full: &str) -> String {
    full.split_whitespace().next().unwrap_or(full).to_string()
}

/// The next items of a practice session: open or resume the session, check the budget, rank,
/// interleave reviews, record impressions.
pub async fn next(
    state: &AppState,
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    session_id: Option<Uuid>,
    limit: Option<u8>,
) -> Result<NextBatch, ApiError> {
    let tenant_id = tx.tenant_id();
    let es = settings(ctx).clone();
    let now = Utc::now();

    let existing = match session_id {
        Some(id) => repo::session(tx.conn(), id)
            .await?
            .filter(|s| s.user_id == ctx.user.id && s.ended_at.is_none()),
        None => None,
    };
    let session = match existing {
        Some(s) => s,
        None => repo::create_session(tx.conn(), tenant_id, ctx.user.id).await?,
    };

    let today_ms = repo::practice_ms_since(
        tx.conn(),
        ctx.user.id,
        local_day_start(&ctx.tenant.timezone, ctx.today),
    )
    .await?;
    let within = ctx.settings.within_windows(&ctx.tenant.timezone, now);
    let mut budget = eng::budget(
        &es,
        Usage {
            today_ms,
            session_ms: session.active_ms,
        },
        within,
        ctx.user.is_paused_on(ctx.today),
    );
    let batch = |items: Vec<NextItem>, budget: &eng::Budget| NextBatch {
        session_id: session.id,
        items,
        budget: budget_view(budget),
        autoplay_next: es.autoplay_next,
        peer_sharing: es.peer_sharing,
    };
    if !budget.open {
        if let Some(stop) = budget.stop {
            repo::end_session(tx.conn(), session.id, stop.as_str()).await?;
        }
        return Ok(batch(vec![], &budget));
    }

    let exclude = repo::session_reels(tx.conn(), session.id).await?;
    let inputs = loader::load(
        tx.conn(),
        &ctx.user,
        &es,
        now,
        ctx.today,
        Scope::Practice {
            open_catalog: es.open_catalog,
            exclude: &exclude,
            cooldown_days: loader::PRACTICE_COOLDOWN_DAYS,
        },
        es.peer_sharing,
    )
    .await?;
    let want = usize::from(limit.unwrap_or(es.batch_size).clamp(1, 20));
    let mut es_for_user = es.clone();
    if !ctx.user.personalized {
        es_for_user.exploration_rate = 0.0;
        es_for_user.social_proof = false;
        es_for_user.weights.interest = 0.0;
    }
    // New people: explore more for their first 20 items (research notes, section (c)).
    if ctx.user.personalized
        && inputs.profile.topic_interest.is_empty()
        && session.items_served < 20
    {
        es_for_user.exploration_rate = (es.exploration_rate * 2.5).min(0.5);
    }
    let rank_ctx = inputs.context(
        &es_for_user,
        now,
        ctx.today,
        [
            ctx.user.id.as_u128(),
            session.id.as_u128(),
            u128::try_from(session.items_served.max(0)).unwrap_or(0),
        ],
    );
    let ranked = rank(&inputs.candidates, &inputs.profile, &rank_ctx, want);
    let shared_names: std::collections::HashMap<Uuid, String> = inputs
        .candidates
        .iter()
        .filter_map(|c| c.shared_by.clone().map(|n| (c.reel_id, n)))
        .collect();

    // Reviews interleaved every `review_every` reels.
    let reviews = if es.review_every > 0 && !ranked.is_empty() {
        let n = (ranked.len() / usize::from(es.review_every)).max(1);
        repo::weak_memories(
            tx.conn(),
            ctx.user.id,
            ctx.today,
            &[],
            i64::try_from(n).unwrap_or(1),
        )
        .await?
    } else {
        vec![]
    };

    let lang = ctx.user.preferred_language.clone();
    let cohort_label = ctx
        .user
        .location
        .clone()
        .or_else(|| ctx.user.job_title.clone());
    let mut items = Vec::with_capacity(ranked.len() + reviews.len());
    let mut impressions = Vec::with_capacity(ranked.len());
    let mut review_iter = reviews.iter();
    let mut position = session.items_served;
    for (i, r) in ranked.iter().enumerate() {
        let Some(reel) = reels::find(tx.conn(), r.reel_id).await? else {
            continue;
        };
        let topic_name = topics::get(tx.conn(), reel.topic_id)
            .await
            .map(|t| t.name)
            .unwrap_or_default();
        let view = views::reel_playable(state, tx, &ctx.user, &reel, &lang, &topic_name).await?;
        let detail = match r.reason {
            RankReason::Shared => shared_names.get(&r.reel_id).map(|n| first_name(n)),
            RankReason::Popular => cohort_label.clone().or_else(|| Some(topic_name.clone())),
            _ => Some(topic_name.clone()),
        };
        impressions.push((
            reel.id,
            i16::try_from(position).unwrap_or(i16::MAX),
            r.score,
            r.reason.as_str().to_string(),
        ));
        items.push(NextItem {
            id: Uuid::now_v7(),
            position,
            kind: PlanItemKind::Reel,
            reason: r.reason.as_str().to_string(),
            reason_detail: detail,
            reel: Some(view),
            question: None,
            review_note: None,
        });
        position += 1;
        if es.review_every > 0 && (i + 1) % usize::from(es.review_every) == 0 {
            if let Some(m) = review_iter.next() {
                if let Some(q) = reels::question(tx.conn(), m.question_id).await? {
                    items.push(NextItem {
                        id: Uuid::now_v7(),
                        position,
                        kind: PlanItemKind::QuestionOnly,
                        reason: "review".into(),
                        reason_detail: None,
                        reel: None,
                        question: Some(views::question(&q, false)),
                        review_note: Some(recall::review_note(&MemoryState::from(m))),
                    });
                    position += 1;
                }
            }
        }
    }
    repo::insert_impressions(tx.conn(), tenant_id, ctx.user.id, session.id, &impressions).await?;
    let served = i32::try_from(items.len()).unwrap_or(0);
    repo::touch_session(tx.conn(), session.id, served, 0, 0).await?;
    if items.is_empty() {
        budget.open = false;
        budget.stop = Some(eng::StopReason::Exhausted);
        repo::end_session(tx.conn(), session.id, "exhausted").await?;
    }
    Ok(batch(items, &budget))
}

pub async fn end_session(tx: &mut TenantTx, ctx: &LearnerCtx, id: Uuid) -> Result<(), ApiError> {
    match repo::session(tx.conn(), id).await? {
        Some(s) if s.user_id == ctx.user.id => {
            repo::end_session(tx.conn(), id, "closed").await?;
            Ok(())
        }
        _ => Err(ApiError::NotFound("session not found".into())),
    }
}

// ---------------------------------------------------------------- the person's topics

pub async fn my_interests(tx: &mut TenantTx, ctx: &LearnerCtx) -> Result<MyInterests, ApiError> {
    let es = settings(ctx);
    let now = Utc::now();
    let rows = repo::my_topics(tx.conn(), ctx.user.id, es.open_catalog).await?;
    let topics = rows
        .into_iter()
        .map(|r| {
            let score = match (r.score, r.last_event_at) {
                (Some(s), Some(at)) => eng::decayed(s, at, now, es.interest_half_life_days),
                _ => 0.0,
            };
            let name = match (ctx.user.preferred_language.as_str(), r.name_ar) {
                ("ar", Some(ar)) if !ar.trim().is_empty() => ar,
                _ => r.name,
            };
            MyTopic {
                topic_id: r.topic_id,
                name,
                is_mandatory: r.is_mandatory,
                assigned: r.assigned,
                preference: r.preference,
                interest: interest_level(score),
                published_reels: r.published_reels,
            }
        })
        .collect();
    let mut tags: Vec<TagInterest> = repo::tag_interest(tx.conn(), ctx.user.id)
        .await?
        .into_iter()
        .map(|t| TagInterest {
            interest: interest_level(eng::decayed(
                t.score,
                t.last_event_at,
                now,
                es.interest_half_life_days,
            )),
            tag: t.tag,
        })
        .filter(|t| t.interest > 0)
        .collect();
    tags.sort_by(|a, b| b.interest.cmp(&a.interest).then(a.tag.cmp(&b.tag)));
    tags.truncate(12);
    Ok(MyInterests {
        personalized: ctx.user.personalized,
        topics,
        top_tags: tags,
    })
}

pub async fn set_topic_preference(
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    topic_id: Uuid,
    preference: Option<&str>,
) -> Result<(), ApiError> {
    let pref = match preference {
        None => None,
        Some(p) => Some(
            Preference::parse(p)
                .ok_or_else(|| ApiError::field("preference", "must be follow, mute or null"))?,
        ),
    };
    let rows = repo::my_topics(tx.conn(), ctx.user.id, true).await?;
    let topic = rows
        .iter()
        .find(|t| t.topic_id == topic_id)
        .ok_or_else(|| ApiError::NotFound("topic not found".into()))?;
    if pref == Some(Preference::Mute) && topic.is_mandatory {
        return Err(ApiError::field(
            "preference",
            "a mandatory topic cannot be muted",
        ));
    }
    let tenant_id = tx.tenant_id();
    repo::set_preference(tx.conn(), tenant_id, ctx.user.id, topic_id, pref).await?;
    Ok(())
}

// ---------------------------------------------------------------- sharing

pub async fn colleagues(
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    q: Option<&str>,
) -> Result<Vec<Colleague>, ApiError> {
    Ok(repo::colleagues(tx.conn(), ctx.user.id, q, 30)
        .await?
        .into_iter()
        .map(|c| Colleague {
            id: c.id,
            full_name: c.full_name,
            job_title: c.job_title,
            location: c.location,
        })
        .collect())
}

pub async fn share(
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    req: &ShareCreate,
) -> Result<ShareResult, ApiError> {
    let es = settings(ctx).clone();
    if !es.peer_sharing {
        return Err(ApiError::Forbidden(
            "sharing with colleagues is switched off for this company".into(),
        ));
    }
    if req.recipient_ids.is_empty() || req.recipient_ids.len() > MAX_RECIPIENTS {
        return Err(ApiError::field(
            "recipient_ids",
            "send to 1 to 10 colleagues",
        ));
    }
    let note = req.note.as_deref().map(str::trim).filter(|n| !n.is_empty());
    if note.map(|n| n.chars().count() > 280).unwrap_or(false) {
        return Err(ApiError::field("note", "must be at most 280 characters"));
    }
    let reel = reels::find(tx.conn(), req.reel_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("reel not found".into()))?;
    if !repo::may_play(tx.conn(), ctx.user.id, reel.id, es.open_catalog).await? {
        return Err(ApiError::NotFound("reel not found".into()));
    }
    let since = local_day_start(&ctx.tenant.timezone, ctx.today);
    let already = repo::shares_sent_since(tx.conn(), ctx.user.id, since).await?;
    if already + i64::try_from(req.recipient_ids.len()).unwrap_or(0) > MAX_SHARES_PER_DAY {
        return Err(ApiError::TooManyRequests {
            retry_after_seconds: 3600,
        });
    }
    let tenant_id = tx.tenant_id();
    let mut sent = 0;
    for rid in &req.recipient_ids {
        if *rid == ctx.user.id {
            continue;
        }
        let Some(recipient) = reelearn_db::repos::users::find(tx.conn(), *rid).await? else {
            continue;
        };
        if recipient.status != reelearn_domain::UserStatus::Active {
            continue;
        }
        // The recipient must be allowed to see it too; a share never widens access.
        if !repo::may_play(tx.conn(), recipient.id, reel.id, es.open_catalog).await? {
            continue;
        }
        if repo::create_share(
            tx.conn(),
            tenant_id,
            reel.id,
            ctx.user.id,
            recipient.id,
            note,
        )
        .await?
        {
            sent += 1;
        }
    }
    if sent > 0 {
        apply_signal(
            tx,
            ctx,
            &reel,
            EngagementKind::Share,
            None,
            None,
            None,
            Channel::App,
        )
        .await?;
    }
    Ok(ShareResult { sent })
}

// ---------------------------------------------------------------- company tuning and insights

pub fn presets() -> std::collections::BTreeMap<String, EngineSettings> {
    eng::Intensity::PRESETS
        .iter()
        .map(|p| (p.as_str().to_string(), EngineSettings::preset(*p)))
        .collect()
}

/// Apply a PUT body to the current settings: `{"intensity": "<preset>"}` alone switches to the
/// preset; anything else is merged field by field (weights too) and re-labelled.
pub fn apply_engine_patch(
    current: &EngineSettings,
    patch: &serde_json::Value,
) -> Result<EngineSettings, ApiError> {
    let obj = patch
        .as_object()
        .ok_or_else(|| ApiError::BadRequest("expected a JSON object".into()))?;
    if obj.len() == 1 {
        if let Some(name) = obj.get("intensity").and_then(|v| v.as_str()) {
            if let Some(p) = eng::Intensity::PRESETS.iter().find(|p| p.as_str() == name) {
                return Ok(EngineSettings::preset(*p));
            }
            if name != "custom" {
                return Err(ApiError::field(
                    "intensity",
                    "must be gentle, balanced, intense, maximum or custom",
                ));
            }
        }
    }
    let mut base = serde_json::to_value(current).map_err(|e| ApiError::Internal(e.into()))?;
    merge(&mut base, patch);
    let next: EngineSettings = serde_json::from_value(base)
        .map_err(|e| ApiError::BadRequest(format!("invalid engine settings: {e}")))?;
    if let Err(errors) = next.validate() {
        return Err(ApiError::Validation(
            errors
                .into_iter()
                .map(|e| crate::error::FieldError {
                    field: e.field,
                    message: e.message,
                })
                .collect(),
        ));
    }
    Ok(next.relabelled())
}

fn merge(base: &mut serde_json::Value, patch: &serde_json::Value) {
    match (base.as_object_mut(), patch.as_object()) {
        (Some(b), Some(p)) => {
            for (k, v) in p {
                if v.is_null() || k == "intensity" {
                    continue;
                }
                match b.get_mut(k) {
                    Some(existing) if existing.is_object() && v.is_object() => merge(existing, v),
                    _ => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        _ => *base = patch.clone(),
    }
}

fn rate(k: i64, n: i64) -> f64 {
    if n > 0 {
        k as f64 / n as f64
    } else {
        0.0
    }
}

fn minutes(ms: i64) -> f64 {
    (ms as f64 / 60_000.0 * 10.0).round() / 10.0
}

pub async fn insights(
    tx: &mut TenantTx,
    settings: &EngineSettings,
    timezone: &str,
    since: DateTime<Utc>,
    range: String,
) -> Result<EngineInsights, ApiError> {
    let totals = repo::session_totals(tx.conn(), since).await?;
    let counts = repo::event_counts(tx.conn(), since).await?;
    let count = |k: &str| {
        counts
            .iter()
            .find(|c| c.kind == k)
            .map(|c| c.events)
            .unwrap_or(0)
    };
    let starts = count("start");
    let shares = repo::share_totals(tx.conn(), since).await?;
    let daily = repo::daily(tx.conn(), since, timezone)
        .await?
        .into_iter()
        .map(|d| InsightDay {
            day: d.day,
            sessions: d.sessions,
            practice_minutes: minutes(d.active_ms),
            starts: d.starts,
            completions: d.completions,
        })
        .collect();
    let topics = repo::topic_interest_agg(tx.conn(), 12)
        .await?
        .into_iter()
        .map(|t| InsightTopic {
            topic_id: t.topic_id,
            name: t.name,
            interested: t.interested,
            followers: t.followers,
            muted: t.muted,
            avg_interest: (t.avg_score * 10.0).round() / 10.0,
        })
        .collect();
    let tags = repo::tag_interest_agg(tx.conn(), 15)
        .await?
        .into_iter()
        .map(|t| InsightTag {
            tag: t.tag,
            interested: t.interested,
            avg_interest: (t.avg_score * 10.0).round() / 10.0,
        })
        .collect();
    let cohorts = repo::cohort_practice(tx.conn(), since, 10)
        .await?
        .into_iter()
        .filter(|c| c.learners >= MIN_GROUP_FOR_REPORTS)
        .map(|c| InsightCohort {
            location: c.key,
            learners: c.learners,
            sessions: c.sessions,
            practice_minutes: minutes(c.active_ms),
        })
        .collect();
    const BAND_LABELS: [&str; 5] = [
        "under 20 s",
        "20–35 s",
        "35–50 s",
        "50–70 s",
        "70 s and over",
    ];
    let bands = repo::band_baselines(tx.conn())
        .await?
        .into_iter()
        .map(|b| InsightBand {
            band: b.band,
            label: BAND_LABELS
                .get(usize::try_from(b.band).unwrap_or(0))
                .copied()
                .unwrap_or("")
                .to_string(),
            starts: i64::from(b.starts),
            completion_rate: rate(i64::from(b.completions), i64::from(b.starts)),
            skip_rate: rate(i64::from(b.skips), i64::from(b.starts)),
        })
        .collect();
    let reasons = repo::reason_mix(tx.conn(), since)
        .await?
        .into_iter()
        .map(|r| InsightReason {
            reason: r.reason,
            impressions: r.impressions,
        })
        .collect();
    let reels = repo::reel_performance(tx.conn(), 12)
        .await?
        .into_iter()
        .map(|r| InsightReel {
            reel_id: r.reel_id,
            title: r.title,
            topic_name: r.topic_name,
            duration_ms: r.duration_ms,
            impressions: i64::from(r.impressions),
            starts: i64::from(r.starts),
            completion_rate: rate(i64::from(r.completions), i64::from(r.starts)),
            skip_rate: rate(i64::from(r.skips), i64::from(r.starts)),
            saves: i64::from(r.saves),
            shares: i64::from(r.shares),
            not_relevant: i64::from(r.not_relevant),
            correct_rate: (r.answers > 0)
                .then(|| rate(i64::from(r.answers_correct), i64::from(r.answers))),
        })
        .collect();
    Ok(EngineInsights {
        range,
        intensity: settings.intensity.as_str().to_string(),
        sessions: totals.sessions,
        learners_practising: totals.learners,
        practice_minutes: minutes(totals.active_ms),
        avg_session_minutes: if totals.sessions > 0 {
            minutes(totals.active_ms / totals.sessions)
        } else {
            0.0
        },
        avg_minutes_per_learner: if totals.learners > 0 {
            minutes(totals.active_ms / totals.learners)
        } else {
            0.0
        },
        items_per_session: (rate(totals.served, totals.sessions) * 10.0).round() / 10.0,
        completion_rate: rate(count("complete"), starts),
        skip_rate: rate(count("skip"), starts),
        saves: count("save"),
        not_relevant: count("not_relevant"),
        sharing: InsightSharing {
            shares: shares.shares,
            senders: shares.senders,
            recipients: shares.recipients,
            opened_rate: rate(shares.opened, shares.shares),
        },
        daily,
        topics,
        tags,
        cohorts,
        bands,
        reasons,
        reels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use reelearn_domain::engine::Intensity;

    #[test]
    fn a_preset_name_alone_switches_preset() {
        let cur = EngineSettings::preset(Intensity::Balanced);
        let next =
            apply_engine_patch(&cur, &serde_json::json!({ "intensity": "maximum" })).expect("ok");
        assert_eq!(next, EngineSettings::preset(Intensity::Maximum));
    }

    #[test]
    fn a_field_edit_merges_and_relabels() {
        let cur = EngineSettings::preset(Intensity::Balanced);
        let next = apply_engine_patch(
            &cur,
            &serde_json::json!({ "intensity": "balanced", "daily_minutes_cap": 25, "weights": { "share": 2.5 } }),
        )
        .expect("ok");
        assert_eq!(next.daily_minutes_cap, 25);
        assert_eq!(next.weights.share, 2.5);
        assert_eq!(
            next.weights.completion, cur.weights.completion,
            "other weights kept"
        );
        assert_eq!(next.intensity, Intensity::Custom);
    }

    #[test]
    fn invalid_values_are_field_errors() {
        let cur = EngineSettings::default();
        match apply_engine_patch(&cur, &serde_json::json!({ "exploration_rate": 0.9 })) {
            Err(ApiError::Validation(v)) => assert_eq!(v[0].field, "exploration_rate"),
            other => panic!("expected validation error, got {other:?}"),
        }
        assert!(apply_engine_patch(&cur, &serde_json::json!({ "intensity": "wild" })).is_err());
    }

    #[test]
    fn interest_levels_are_bounded() {
        assert_eq!(interest_level(-5.0), 0);
        assert_eq!(interest_level(0.0), 0);
        assert!(interest_level(5.0) > 40 && interest_level(5.0) < 50);
        assert_eq!(interest_level(50.0), 100);
    }
}
