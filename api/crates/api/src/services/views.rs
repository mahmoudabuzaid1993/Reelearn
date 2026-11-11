//! Build wire views from rows. Learner views never carry `correct_key`.

use crate::dto::*;
use crate::error::ApiError;
use crate::services::media::signed_url;
use crate::state::AppState;
use reelearn_db::repos::{reels, stats, tenants::TenantRow, topics, users, users::UserRow};
use reelearn_db::TenantTx;
use reelearn_domain::ReelStatus;

pub fn tenant_public(t: &TenantRow, logo_url: Option<String>) -> TenantPublic {
    TenantPublic {
        id: t.id,
        slug: t.slug.clone(),
        name: t.name.clone(),
        logo_url,
        languages: t.languages.clone(),
        region: t.region,
    }
}

pub async fn tenant_public_signed(state: &AppState, t: &TenantRow) -> TenantPublic {
    let logo = signed_url(state, t.logo_key.as_deref()).await;
    tenant_public(t, logo)
}

pub fn tenant_full(t: &TenantRow, public: TenantPublic) -> Tenant {
    Tenant {
        public,
        default_language: t.default_language.clone(),
        timezone: t.timezone.clone(),
        credits: t.credits,
        settings: reelearn_db::TenantSettings::from_json(&t.settings).to_json(),
    }
}

pub async fn me(
    state: &AppState,
    tx: &mut TenantTx,
    user: &UserRow,
    tenant: &TenantRow,
) -> Result<Me, ApiError> {
    let is_first_open =
        !users::has_devices(tx.conn(), user.id).await? || user.last_seen_at.is_none();
    Ok(Me {
        id: user.id,
        tenant: tenant_public_signed(state, tenant).await,
        role: user.role,
        full_name: user.full_name.clone(),
        first_name: user.first_name(),
        phone_e164: user.phone_e164.clone(),
        email: user.email.clone(),
        preferred_language: user.preferred_language.clone(),
        ui_language: user.ui_language.clone(),
        delivery_channel: user.delivery_channel,
        delivery_slot: user.delivery_slot,
        captions_on: user.captions_on,
        reminders_on: user.reminders_on,
        extra_reels_on: user.extra_reels_on,
        paused_until: user.paused_until,
        is_first_open,
    })
}

pub async fn user(tx: &mut TenantTx, u: &UserRow) -> Result<User, ApiError> {
    let groups = users::groups_of(tx.conn(), u.id)
        .await?
        .into_iter()
        .map(|g| GroupRef {
            id: g.id,
            name: g.name,
        })
        .collect();
    Ok(User {
        id: u.id,
        role: u.role,
        status: u.status,
        full_name: u.full_name.clone(),
        phone_e164: u.phone_e164.clone(),
        email: u.email.clone(),
        employee_no: u.employee_no.clone(),
        preferred_language: u.preferred_language.clone(),
        manager_id: u.manager_id,
        location: u.location.clone(),
        job_title: u.job_title.clone(),
        delivery_channel: u.delivery_channel,
        delivery_slot: u.delivery_slot,
        groups,
        last_seen_at: u.last_seen_at,
        created_at: u.created_at,
    })
}

pub fn topic(t: &topics::TopicRow) -> Topic {
    Topic {
        id: t.id,
        slug: t.slug.clone(),
        name: t.name.clone(),
        name_ar: t.name_ar.clone(),
        description: t.description.clone(),
        compliance_tag: t.compliance_tag.clone(),
        is_mandatory: t.is_mandatory,
        status: t.status.clone(),
        reel_counts: ReelCounts {
            published: t.published_count,
            in_review: t.in_review_count,
            draft: t.draft_count,
        },
        recall_pct: t.recall_pct,
    }
}

pub fn assignment(a: &topics::AssignmentRow) -> Assignment {
    Assignment {
        id: a.id,
        topic_id: a.topic_id,
        group: GroupRef {
            id: a.group_id,
            name: a.group_name.clone(),
        },
        reels_per_day: a.reels_per_day,
        channel: a.channel,
        is_mandatory: a.is_mandatory,
        starts_on: a.starts_on,
        ends_on: a.ends_on,
    }
}

pub fn question(q: &reels::QuestionRow, with_answer: bool) -> Question {
    Question {
        id: q.id,
        language: q.language.clone(),
        prompt: q.prompt.clone(),
        options: parse_options(&q.options),
        correct_key: with_answer.then(|| q.correct_key.clone()),
        explanation: q.explanation.clone(),
    }
}

pub fn user_stats(s: Option<&stats::UserStatsRow>) -> UserStats {
    match s {
        Some(s) => UserStats {
            streak_current: s.streak_current,
            streak_longest: s.streak_longest,
            freezes_available: s.freezes_available,
            remembered_count: s.remembered_count,
            reels_completed: s.reels_completed,
            last_active_on: s.last_active_on,
        },
        None => UserStats::default(),
    }
}

/// Full admin view of a reel: question with `correct_key`, approvals, audience, signed URLs.
pub async fn reel_admin(
    state: &AppState,
    tx: &mut TenantTx,
    r: &reels::ReelRow,
) -> Result<Reel, ApiError> {
    let variants_rows = reels::variants(tx.conn(), r.id).await?;
    let mut variants = Vec::with_capacity(variants_rows.len());
    for v in &variants_rows {
        variants.push(VariantView {
            language: v.language.clone(),
            dub_status: v.dub_status,
            video_url: signed_url(state, v.video_key.as_deref()).await,
            captions_url: signed_url(state, v.captions_key.as_deref()).await,
        });
    }
    let q = reels::question_for_reel_in(tx.conn(), r.id, &r.language, &r.language).await?;
    let approvals = reels::approvals(tx.conn(), r.id)
        .await?
        .into_iter()
        .map(|a| ApprovalView {
            role: a.role,
            decision: a.decision,
            reviewer_name: a.reviewer_name,
            comment: a.comment,
            decided_at: a.decided_at,
            reel_version: a.reel_version,
        })
        .collect();
    let audience = reels::audience(tx.conn(), r.id)
        .await?
        .into_iter()
        .map(|g| GroupRef {
            id: g.id,
            name: g.name,
        })
        .collect();
    let recall = if r.status == ReelStatus::Published {
        reels::recall_pct(tx.conn(), r.id).await?
    } else {
        None
    };
    Ok(Reel {
        id: r.id,
        topic_id: r.topic_id,
        source_id: r.source_id,
        generation_job_id: r.generation_job_id,
        kind: r.kind,
        status: r.status,
        title: r.title.clone(),
        language: r.language.clone(),
        script: r.script.clone(),
        source_citation: r.source_citation.clone(),
        presenter: r.presenter.clone(),
        video_url: signed_url(state, r.video_key.as_deref()).await,
        thumbnail_url: signed_url(state, r.thumbnail_key.as_deref()).await,
        captions_url: signed_url(state, r.captions_key.as_deref()).await,
        duration_ms: r.duration_ms,
        version: r.version,
        compliance_tag: r.compliance_tag.clone(),
        consent_confirmed: r.consent_confirmed,
        variants,
        question: q.map(|q| question(&q, true)),
        approvals,
        audience,
        recall_pct: recall,
        published_at: r.published_at,
        created_at: r.created_at,
        updated_at: r.updated_at,
        quality: ReelQuality::from_review(r.ai_review.as_ref(), r.quality_score),
    })
}

pub async fn reel_list_item(
    state: &AppState,
    tx: &mut TenantTx,
    r: &reels::ReelRow,
    topic_name: &str,
) -> Result<ReelListItem, ApiError> {
    let variants = reels::variants(tx.conn(), r.id).await?;
    let mut languages = vec![r.language.clone()];
    languages.extend(
        variants
            .iter()
            .map(|v| v.language.clone())
            .filter(|l| l != &r.language),
    );
    let (recall, plays) = if r.status == ReelStatus::Published {
        (
            reels::recall_pct(tx.conn(), r.id).await?,
            reels::plays(tx.conn(), r.id).await?,
        )
    } else {
        (None, 0)
    };
    let awaiting = if r.status == ReelStatus::InReview {
        let roles = reels::approved_roles_for_version(tx.conn(), r.id, r.version).await?;
        let has_ld = roles.contains(&reelearn_domain::ApprovalRole::Ld);
        let has_sme = roles.contains(&reelearn_domain::ApprovalRole::Sme);
        match (has_ld, has_sme) {
            (true, true) => None,
            (false, true) => Some("L&D sign-off".to_string()),
            (true, false) => Some("SME sign-off".to_string()),
            (false, false) => Some("L&D and SME sign-off".to_string()),
        }
    } else {
        None
    };
    Ok(ReelListItem {
        id: r.id,
        topic: TopicRef {
            id: r.topic_id,
            name: topic_name.to_string(),
        },
        kind: r.kind,
        status: r.status,
        title: r.title.clone(),
        language: r.language.clone(),
        languages,
        duration_ms: r.duration_ms,
        thumbnail_url: signed_url(state, r.thumbnail_key.as_deref()).await,
        version: r.version,
        recall_pct: recall,
        plays,
        awaiting,
        updated_at: r.updated_at,
        quality: ReelQuality::from_review(r.ai_review.as_ref(), r.quality_score),
    })
}

/// Learner playback view in `lang` (variant if dubbed, else master). Never includes `correct_key`.
pub async fn reel_playable(
    state: &AppState,
    tx: &mut TenantTx,
    user: &UserRow,
    r: &reels::ReelRow,
    lang: &str,
    topic_name: &str,
) -> Result<ReelPlayable, ApiError> {
    let variant = if lang != r.language {
        reels::variant(tx.conn(), r.id, lang).await?.filter(|v| {
            v.dub_status == reelearn_domain::ExtractionStatus::Done && v.video_key.is_some()
        })
    } else {
        None
    };
    let (video_key, captions_key, language, script) = match &variant {
        Some(v) => (
            v.video_key.clone(),
            v.captions_key.clone().or_else(|| r.captions_key.clone()),
            v.language.clone(),
            v.script.clone().or_else(|| r.script.clone()),
        ),
        None => (
            r.video_key.clone(),
            r.captions_key.clone(),
            r.language.clone(),
            r.script.clone(),
        ),
    };
    let q = reels::question_for_reel_in(tx.conn(), r.id, lang, &r.language).await?;
    let group = reels::assigning_group_name(tx.conn(), user.id, r.id).await?;
    let why = match (&group, &r.source_citation) {
        (Some(g), Some(c)) => format!("{topic_name} · assigned to {g} · from {c}"),
        (Some(g), None) => format!("{topic_name} · assigned to {g}"),
        (None, Some(c)) => format!("{topic_name} · from {c}"),
        (None, None) => topic_name.to_string(),
    };
    let saved = reelearn_db::repos::learning::is_saved(tx.conn(), user.id, r.id).await?;
    let video_url = signed_url(state, video_key.as_deref())
        .await
        .unwrap_or_default();
    Ok(ReelPlayable {
        id: r.id,
        title: r.title.clone(),
        topic_name: topic_name.to_string(),
        presenter_name: r.presenter_name(),
        language,
        duration_ms: r.duration_ms.unwrap_or(30_000),
        video_url,
        captions_url: signed_url(state, captions_key.as_deref()).await,
        thumbnail_url: signed_url(state, r.thumbnail_key.as_deref()).await,
        captions_on_by_default: user.captions_on,
        caption_text: script.as_deref().map(key_line),
        source_citation: r.source_citation.clone(),
        why_am_i_seeing_this: why,
        question: q.map(|q| question(&q, false)),
        saved,
    })
}

/// The first sentence of the script is the on-screen caption line.
fn key_line(script: &str) -> String {
    let first = script
        .split(['.', '\n', '؟', '!', '?'])
        .next()
        .unwrap_or(script)
        .trim();
    first.chars().take(120).collect()
}
