//! `/me`: profile, preferences, privacy statement, personal data export.

use crate::auth::AuthUser;
use crate::dto::*;
use crate::error::ApiError;
use crate::routes::Json;
use crate::services::{learn, views};
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use reelearn_db::repos::{exports, jobs, tenants, users};
use reelearn_domain::ExportKind;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_me).patch(patch_me))
        .route("/me/privacy", get(privacy))
        .route("/me/export", post(request_export))
}

async fn get_me(State(state): State<AppState>, auth: AuthUser) -> Result<Json<Me>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    let user = users::get(tx.conn(), auth.user_id).await?;
    let me = views::me(&state, &mut tx, &user, &tenant).await?;
    tx.commit().await?;
    Ok(Json(me))
}

async fn patch_me(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<MePatch>,
) -> Result<Json<Me>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let tenant = tenants::get(tx.conn(), auth.tenant_id).await?;
    if let Some(lang) = body.preferred_language.as_deref() {
        if !tenant.languages.iter().any(|l| l == lang) {
            return Err(ApiError::field(
                "preferred_language",
                "is not one of the workspace languages",
            ));
        }
    }
    let update = users::UserUpdate {
        preferred_language: body.preferred_language,
        ui_language: body.ui_language,
        delivery_channel: body.delivery_channel,
        delivery_slot: body.delivery_slot,
        captions_on: body.captions_on,
        reminders_on: body.reminders_on,
        extra_reels_on: body.extra_reels_on,
        paused_until: body.paused_until,
        ..Default::default()
    };
    let user = users::update(tx.conn(), auth.user_id, update).await?;
    if let Some(token) = body.push_token.as_deref() {
        users::set_push_token(tx.conn(), auth.user_id, token).await?;
    }
    users::touch_last_seen(tx.conn(), auth.user_id).await?;
    let me = views::me(&state, &mut tx, &user, &tenant).await?;
    tx.commit().await?;
    learn::invalidate_plan_cache(&state, auth.tenant_id, auth.user_id).await;
    Ok(Json(me))
}

async fn privacy(State(state): State<AppState>, auth: AuthUser) -> Result<Json<Privacy>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let user = users::get(tx.conn(), auth.user_id).await?;
    tx.commit().await?;
    let p = if user.ui_language == "ar" {
        Privacy {
            manager_sees: vec![
                "جاهزيتك في كل موضوع (جاهز / ليس بعد / لم يبدأ)".into(),
                "متوسط التذكر للفريق".into(),
                "آخر يوم نشاط".into(),
            ],
            hr_sees: vec![
                "الساعات على المواضيع الإلزامية".into(),
                "إجماليات حسب الموضوع والمجموعة".into(),
            ],
            nobody_sees: vec![
                "أي إجابة اخترتها".into(),
                "ما حفظته".into(),
                "متى تشاهد".into(),
                "موقعك".into(),
            ],
        }
    } else {
        Privacy {
            manager_sees: vec![
                "Your readiness per topic (ready / not yet / not started)".into(),
                "The team's average recall".into(),
                "Your last active day".into(),
            ],
            hr_sees: vec![
                "Hours on mandatory topics".into(),
                "Totals per topic and per group".into(),
            ],
            nobody_sees: vec![
                "Which answer you chose".into(),
                "What you saved".into(),
                "When you watch".into(),
                "Your location".into(),
            ],
        }
    };
    Ok(Json(p))
}

async fn request_export(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<(StatusCode, Json<Export>), ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let row = exports::create(
        tx.conn(),
        auth.tenant_id,
        ExportKind::PersonalData,
        auth.user_id,
        Some(auth.user_id),
        None,
        None,
    )
    .await?;
    jobs::enqueue(
        tx.conn(),
        Some(auth.tenant_id),
        "data_export",
        serde_json::json!({ "export_id": row.id, "user_id": auth.user_id }),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(export_view(&row))))
}

pub fn export_view(r: &exports::ExportRow) -> Export {
    Export {
        id: r.id,
        kind: r.kind,
        status: r.status,
        period_start: r.period_start,
        period_end: r.period_end,
        created_at: r.created_at,
        finished_at: r.finished_at,
    }
}
