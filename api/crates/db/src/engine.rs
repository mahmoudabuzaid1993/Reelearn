//! Loads what the learning engine ranks (candidates, the person's interest and ability, the
//! company's duration baselines, colleague activity and shares) and turns the rows into the
//! pure types of `reelearn_domain::engine`. Shared by the daily plan, `/me/next` and the worker.

use crate::error::DbError;
use crate::repos::engine::{self as repo, FeatureRow, Scope};
use crate::repos::users::UserRow;
use chrono::{DateTime, NaiveDate, Utc};
use reelearn_domain::engine::rank::BAND_COUNT;
use reelearn_domain::engine::{
    decayed, BandBaseline, EngineSettings, ItemStats, Preference, RankCandidate, RankContext,
    UserProfile,
};
use sqlx::PgConnection;
use std::collections::HashMap;
use uuid::Uuid;

/// Reels finished within this many days are left out of practice.
pub const PRACTICE_COOLDOWN_DAYS: i32 = 3;

fn u(v: i32) -> u32 {
    u32::try_from(v.max(0)).unwrap_or(0)
}

pub fn candidate_from(
    row: &FeatureRow,
    cohort: &HashMap<Uuid, f32>,
    role_fit: &HashMap<Uuid, f32>,
    shared_by: Option<String>,
) -> RankCandidate {
    RankCandidate {
        reel_id: row.reel_id,
        topic_id: row.topic_id,
        tags: row.tags.clone(),
        duration_ms: row.duration_ms,
        published_at: row.published_at,
        mandatory: row.mandatory,
        assigned: row.assigned,
        deadline: row.deadline,
        difficulty: row.difficulty,
        has_question: row.has_question,
        stats: ItemStats {
            impressions: u(row.impressions),
            starts: u(row.starts),
            skips: u(row.skips),
            completions: u(row.completions),
            rewatches: u(row.rewatches),
            saves: u(row.saves),
            shares: u(row.shares),
            not_relevant: u(row.not_relevant),
            more_like_this: u(row.more_like_this),
            answers: u(row.answers),
            answers_correct: u(row.answers_correct),
        },
        cohort_popularity: cohort.get(&row.reel_id).copied().unwrap_or(0.0),
        completed_before: row.completed_before,
        shared_by,
        role_fit: role_fit.get(&row.reel_id).copied(),
    }
}

/// The person's interest (decayed to `now`), follow/mute choices and answer record.
pub async fn profile(
    conn: &mut PgConnection,
    user_id: Uuid,
    settings: &EngineSettings,
    now: DateTime<Utc>,
) -> Result<UserProfile, DbError> {
    let hl = settings.interest_half_life_days;
    let mut p = UserProfile::default();
    for t in repo::topic_interest(&mut *conn, user_id).await? {
        p.topic_interest
            .insert(t.topic_id, decayed(t.score, t.last_event_at, now, hl));
        p.signals = p.signals.saturating_add(u(t.events));
        if let Some(pref) = t.preference.as_deref().and_then(Preference::parse) {
            p.topic_preference.insert(t.topic_id, pref);
        }
    }
    for t in repo::tag_interest(&mut *conn, user_id).await? {
        p.tag_interest
            .insert(t.tag, decayed(t.score, t.last_event_at, now, hl));
    }
    let totals = repo::answer_totals(conn, user_id).await?;
    p.answers = u32::try_from(totals.answers.max(0)).unwrap_or(0);
    p.answers_correct = u32::try_from(totals.correct.max(0)).unwrap_or(0);
    Ok(p)
}

pub async fn bands(conn: &mut PgConnection) -> Result<[BandBaseline; BAND_COUNT], DbError> {
    let mut out = [BandBaseline::default(); BAND_COUNT];
    for b in repo::band_baselines(conn).await? {
        if let Some(slot) = usize::try_from(b.band).ok().and_then(|i| out.get_mut(i)) {
            *slot = BandBaseline {
                starts: u(b.starts),
                completions: u(b.completions),
                skips: u(b.skips),
            };
        }
    }
    Ok(out)
}

/// Everything one ranking call needs, owned, so the context can borrow it.
pub struct RankInputs {
    pub candidates: Vec<RankCandidate>,
    pub profile: UserProfile,
    pub bands: [BandBaseline; BAND_COUNT],
}

impl RankInputs {
    pub fn context<'a>(
        &self,
        settings: &'a EngineSettings,
        now: DateTime<Utc>,
        today: NaiveDate,
        seed: [u128; 3],
    ) -> RankContext<'a> {
        RankContext {
            settings,
            now,
            today,
            bands: self.bands,
            seed,
        }
    }
}

pub async fn load(
    conn: &mut PgConnection,
    user: &UserRow,
    settings: &EngineSettings,
    now: DateTime<Utc>,
    day: NaiveDate,
    scope: Scope<'_>,
    with_shares: bool,
) -> Result<RankInputs, DbError> {
    let rows = repo::features(&mut *conn, user.id, day, scope).await?;
    let cohort: HashMap<Uuid, f32> = if settings.social_proof && user.personalized {
        repo::cohort_popularity(
            &mut *conn,
            user.location.as_deref(),
            user.job_title.as_deref(),
            user.manager_id,
        )
        .await?
        .into_iter()
        .collect()
    } else {
        HashMap::new()
    };
    let inbox: HashMap<Uuid, String> = if with_shares {
        repo::share_inbox(&mut *conn, user.id)
            .await?
            .into_iter()
            .map(|r| (r.reel_id, r.sender_name))
            .collect()
    } else {
        HashMap::new()
    };
    // The job-title prior is personalisation too: off when the person switched it off.
    let role_fit: HashMap<Uuid, f32> = match user
        .job_title
        .as_deref()
        .and_then(reelearn_domain::ai::role_key)
    {
        Some(key) if user.personalized => crate::repos::ai::role_fit(&mut *conn, &key)
            .await?
            .into_iter()
            .collect(),
        _ => HashMap::new(),
    };
    let candidates = rows
        .iter()
        .map(|r| candidate_from(r, &cohort, &role_fit, inbox.get(&r.reel_id).cloned()))
        .collect();
    let mut profile = profile(&mut *conn, user.id, settings, now).await?;
    if !user.personalized {
        // Keep only the person's explicit follow/mute choices and answer record.
        profile.topic_interest.clear();
        profile.tag_interest.clear();
    }
    let bands = bands(conn).await?;
    Ok(RankInputs {
        candidates,
        profile,
        bands,
    })
}

/// Value-model scores for the daily plan's new-reel candidates.
pub async fn plan_scores(
    conn: &mut PgConnection,
    user: &UserRow,
    settings: &EngineSettings,
    day: NaiveDate,
    reel_ids: &[Uuid],
) -> Result<HashMap<Uuid, f32>, DbError> {
    if reel_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let now = Utc::now();
    let inputs = load(conn, user, settings, now, day, Scope::Ids(reel_ids), false).await?;
    // Scoring only: no exploration draw, so the seed is irrelevant here.
    let ctx = inputs.context(settings, now, day, [user.id.as_u128(), 0, 0]);
    Ok(inputs
        .candidates
        .iter()
        .map(|c| {
            let (b, _) = reelearn_domain::engine::score(c, &inputs.profile, &ctx);
            (c.reel_id, b.total())
        })
        .collect())
}
