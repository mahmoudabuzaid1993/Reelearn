//! `hrdf_export` (evidence pack CSV for a period) and `data_export` (a learner's personal data as JSON).

use crate::jobs::{JobError, JobOutcome};
use crate::Ctx;
use chrono::Utc;
use reelearn_api::s3::export_key;
use reelearn_db::repos::{exports, learning, stats, topics, users};
use reelearn_domain::JobStatus;
use uuid::Uuid;

pub async fn hrdf_export(
    ctx: &Ctx,
    tenant_id: Uuid,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let export_id = payload_uuid(payload, "export_id")?;
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let row = exports::get(tx.conn(), export_id).await?;
    let (from, to) = match (row.period_start, row.period_end) {
        (Some(f), Some(t)) if t >= f => (f, t),
        _ => {
            exports::finish(
                tx.conn(),
                export_id,
                JobStatus::Failed,
                None,
                Some("a period is required"),
            )
            .await?;
            tx.commit().await?;
            return Err(JobError::Permanent("export has no valid period".into()));
        }
    };
    let rows = stats::hrdf_rows(tx.conn(), from, to).await?;
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record([
        "day",
        "topic",
        "compliance_tag",
        "group",
        "learners_active",
        "reels_completed",
        "answers_total",
        "answers_correct",
        "recall_pct",
        "training_minutes",
    ])
    .map_err(|e| JobError::Permanent(e.to_string()))?;
    for r in &rows {
        let recall = if r.answers_total > 0 {
            (100 * r.answers_correct) / r.answers_total
        } else {
            0
        };
        wtr.write_record([
            r.day.to_string(),
            r.topic.clone(),
            r.compliance_tag.clone().unwrap_or_default(),
            r.group_name.clone(),
            r.learners_active.to_string(),
            r.reels_completed.to_string(),
            r.answers_total.to_string(),
            r.answers_correct.to_string(),
            recall.to_string(),
            format!("{:.2}", r.minutes_total),
        ])
        .map_err(|e| JobError::Permanent(e.to_string()))?;
    }
    let bytes = wtr
        .into_inner()
        .map_err(|e| JobError::Permanent(e.to_string()))?;
    let key = export_key(tenant_id, export_id, "csv");
    match ctx
        .state
        .storage
        .put_bytes(&ctx.state.http, &key, "text/csv", bytes)
        .await
    {
        Ok(()) => {
            exports::finish(tx.conn(), export_id, JobStatus::Done, Some(&key), None).await?;
            tx.commit().await?;
            Ok(JobOutcome::done(format!("{} rows → {key}", rows.len())))
        }
        Err(e) => {
            tx.rollback().await?;
            Err(JobError::Transient(format!("object storage: {e}")))
        }
    }
}

pub async fn data_export(
    ctx: &Ctx,
    tenant_id: Uuid,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let export_id = payload_uuid(payload, "export_id")?;
    let user_id = payload_uuid(payload, "user_id")?;
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let user = users::get(tx.conn(), user_id).await?;
    let user_stats = stats::user_stats(tx.conn(), user_id).await?;
    let topics_all = topics::list(tx.conn()).await?;
    let memory = learning::memory_by_topic(tx.conn(), user_id, Utc::now().date_naive()).await?;
    let by_topic: Vec<serde_json::Value> = memory
        .iter()
        .map(|m| {
            let name = topics_all.iter().find(|t| t.id == m.topic_id).map(|t| t.name.clone()).unwrap_or_default();
            serde_json::json!({ "topic": name, "questions_tracked": m.total, "remembered": m.remembered, "next_review_on": m.next_due_on })
        })
        .collect();
    let engine = personal_engine_data(&mut tx, user_id, &topics_all).await?;
    let doc = serde_json::json!({
        "exported_at": Utc::now(),
        "learning_engine": engine,
        "profile": {
            "full_name": user.full_name, "phone": user.phone_e164, "email": user.email, "employee_no": user.employee_no,
            "preferred_language": user.preferred_language, "delivery_channel": user.delivery_channel, "delivery_slot": user.delivery_slot,
            "captions_on": user.captions_on, "reminders_on": user.reminders_on, "paused_until": user.paused_until,
        },
        "stats": user_stats,
        "learning_by_topic": by_topic,
        "what_your_company_sees": {
            "manager": "whether you are ready on each topic, and the team average",
            "hr": "training minutes on mandatory topics",
            "nobody": "which answers you got wrong, what you saved, when you watch, your location",
            "used_only_to_choose_your_reels": "your interest in topics and tags, your skips and \"not for me\" taps, what colleagues sent you"
        }
    });
    let bytes = serde_json::to_vec_pretty(&doc).map_err(|e| JobError::Permanent(e.to_string()))?;
    let key = export_key(tenant_id, export_id, "json");
    match ctx
        .state
        .storage
        .put_bytes(&ctx.state.http, &key, "application/json", bytes)
        .await
    {
        Ok(()) => {
            exports::finish(tx.conn(), export_id, JobStatus::Done, Some(&key), None).await?;
            tx.commit().await?;
            Ok(JobOutcome::done(format!("personal data → {key}")))
        }
        Err(e) => {
            tx.rollback().await?;
            Err(JobError::Transient(format!("object storage: {e}")))
        }
    }
}

pub fn payload_uuid(payload: &serde_json::Value, key: &str) -> Result<Uuid, JobError> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| JobError::Permanent(format!("payload is missing {key}")))
}

/// What the learning engine holds about one person: interest per topic and tag, follow/mute
/// choices, signal counts, practice sessions and colleague shares (sent and received).
async fn personal_engine_data(
    tx: &mut reelearn_db::TenantTx,
    user_id: Uuid,
    topics_all: &[reelearn_db::repos::topics::TopicRow],
) -> Result<serde_json::Value, JobError> {
    use reelearn_db::repos::engine as eng;
    let name = |id: Uuid| {
        topics_all
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.clone())
            .unwrap_or_default()
    };
    let topics: Vec<serde_json::Value> = eng::topic_interest(tx.conn(), user_id)
        .await?
        .into_iter()
        .map(|t| serde_json::json!({ "topic": name(t.topic_id), "interest_score": t.score, "signals": t.events, "last_signal_at": t.last_event_at, "your_choice": t.preference }))
        .collect();
    let tags: Vec<serde_json::Value> = eng::tag_interest(tx.conn(), user_id)
        .await?
        .into_iter()
        .map(|t| serde_json::json!({ "tag": t.tag, "interest_score": t.score, "last_signal_at": t.last_event_at }))
        .collect();
    let signals: Vec<(String, i64)> = sqlx::query_as(
        "SELECT kind::text, count(*)::bigint FROM engagement_events WHERE user_id = $1 GROUP BY kind ORDER BY kind",
    )
    .bind(user_id)
    .fetch_all(tx.conn())
    .await?;
    let sessions: (i64, i64) = sqlx::query_as(
        "SELECT count(*)::bigint, coalesce(sum(active_ms), 0)::bigint FROM engine_sessions WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(tx.conn())
    .await?;
    let shares: Vec<(String, String, String, Option<chrono::DateTime<Utc>>)> = sqlx::query_as(
        "SELECT CASE WHEN s.sender_id = $1 THEN 'sent' ELSE 'received' END, r.title,
                CASE WHEN s.sender_id = $1 THEN rc.full_name ELSE sd.full_name END, s.created_at
         FROM reel_shares s JOIN reels r ON r.id = s.reel_id
         JOIN users sd ON sd.id = s.sender_id JOIN users rc ON rc.id = s.recipient_id
         WHERE s.sender_id = $1 OR s.recipient_id = $1 ORDER BY s.created_at",
    )
    .bind(user_id)
    .fetch_all(tx.conn())
    .await?;
    Ok(serde_json::json!({
        "topics": topics,
        "tags": tags,
        "signals": signals.into_iter().map(|(k, n)| serde_json::json!({ "kind": k, "count": n })).collect::<Vec<_>>(),
        "practice": { "sessions": sessions.0, "minutes": sessions.1 as f64 / 60_000.0 },
        "colleague_shares": shares.into_iter().map(|(dir, title, who, at)| serde_json::json!({ "direction": dir, "reel": title, "colleague": who, "at": at })).collect::<Vec<_>>(),
    }))
}
