//! Daily plan materialisation: shared by `GET /me/today` (build on miss) and the
//! worker's `build_daily_plans` job. Idempotent per (user, day) thanks to the unique
//! index on `daily_plans (tenant_id, user_id, plan_date)`.

use crate::error::DbError;
use crate::repos::{learning, plans as plan_repo, reels, users::UserRow};
use crate::settings::TenantSettings;
use chrono::NaiveDate;
use reelearn_domain::plan::{build_plan, DueReview, NewReelCandidate, PlanInput};
use reelearn_domain::PlanItemKind;
use sqlx::PgConnection;
use uuid::Uuid;

pub struct MaterializedPlan {
    pub plan: plan_repo::PlanRow,
    pub items: Vec<plan_repo::PlanItemRow>,
    pub created: bool,
}

/// Return today's plan, building and freezing it when it does not exist yet.
pub async fn ensure_plan(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    settings: &TenantSettings,
    user: &UserRow,
    day: NaiveDate,
) -> Result<MaterializedPlan, DbError> {
    if let Some(plan) = plan_repo::find_for_date(&mut *conn, user.id, day).await? {
        let items = plan_repo::items(conn, plan.id).await?;
        return Ok(MaterializedPlan {
            plan,
            items,
            created: false,
        });
    }

    let mut specs = Vec::new();
    if !user.is_paused_on(day) {
        let due = learning::due_reviews(&mut *conn, user.id, day).await?;
        let candidates = reels::unseen_candidates(&mut *conn, user.id, day).await?;

        let mut input = PlanInput {
            today: day,
            max_reviews_per_day: usize::try_from(settings.max_reviews_per_day.max(0)).unwrap_or(1),
            reels_per_day: usize::try_from(settings.reels_per_day.max(0)).unwrap_or(2),
            extra_reels_on: user.extra_reels_on,
            due_reviews: due
                .iter()
                .map(|d| DueReview {
                    question_id: d.question_id,
                    reel_id: d.reel_id,
                    due_on: d.due_on,
                    interval_days: d.interval_days,
                })
                .collect(),
            candidates: Vec::with_capacity(candidates.len()),
        };
        for c in &candidates {
            let question = reels::question_for_reel_in(
                &mut *conn,
                c.reel_id,
                &user.preferred_language,
                &c.language,
            )
            .await?;
            input.candidates.push(NewReelCandidate {
                reel_id: c.reel_id,
                question_id: question.map(|q| q.id),
                topic_id: c.topic_id,
                assignment_id: c.assignment_id,
                assignment_reels_per_day: c.reels_per_day,
                assignment_channel: c.channel,
                is_mandatory: c.is_mandatory,
                published_at: c.published_at,
            });
        }
        specs = build_plan(&input);
    }

    let plan = plan_repo::create(
        &mut *conn,
        tenant_id,
        user.id,
        day,
        i16::try_from(specs.len()).unwrap_or(i16::MAX),
    )
    .await?;
    let mut items = Vec::with_capacity(specs.len());
    for s in &specs {
        let reel_id = match s.kind {
            PlanItemKind::Reel => s.reel_id,
            PlanItemKind::QuestionOnly => s.reel_id,
        };
        items.push(
            plan_repo::insert_item(
                &mut *conn,
                tenant_id,
                plan.id,
                s.position,
                s.kind,
                s.reason,
                reel_id,
                s.question_id,
            )
            .await?,
        );
    }
    Ok(MaterializedPlan {
        plan,
        items,
        created: true,
    })
}
