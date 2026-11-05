//! Worker end-to-end tests with the fake providers: the studio pipeline, daily plan
//! building with WhatsApp scheduling, nudge delivery, and the HRDF export.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use reelearn_api::seed::seed_demo;
use reelearn_api::{build_router, AppState, Config};
use reelearn_db::repos::{deliveries, jobs, plans, users};
use reelearn_worker::providers::fake::FakeWhatsApp;
use reelearn_worker::providers::Providers;
use reelearn_worker::{drain, Ctx};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

struct Harness {
    router: Router,
    ctx: Ctx,
    whatsapp: Arc<FakeWhatsApp>,
}

async fn harness(pool: PgPool) -> Harness {
    let prefix = format!("wtest:{}:", uuid::Uuid::new_v4().simple());
    let state = AppState::for_tests(Config::for_tests("postgres://unused"), pool, &prefix)
        .await
        .expect("state");
    seed_demo(&state).await.expect("seed");
    let _ = state.storage.ensure_bucket(&state.http).await;
    let whatsapp = Arc::new(FakeWhatsApp::default());
    let mut providers = Providers::fake();
    providers.whatsapp = whatsapp.clone();
    Harness {
        router: build_router(state.clone()),
        ctx: Ctx::new(state, providers),
        whatsapp,
    }
}

async fn call(
    h: &Harness,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(format!("/v1{path}"));
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .expect("request");
    let resp = h.router.clone().oneshot(req).await.expect("response");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, json)
}

async fn login(h: &Harness, phone: &str) -> String {
    let (_, v) = call(
        h,
        Method::POST,
        "/auth/otp/request",
        None,
        Some(json!({ "tenant_slug": "alwadi", "phone": phone })),
    )
    .await;
    let challenge = v["challenge_id"].as_str().expect("challenge").to_string();
    let (s, v) = call(h, Method::POST, "/auth/otp/verify", None, Some(json!({ "challenge_id": challenge, "code": "1234", "device": { "device_id": "t", "platform": "web" } }))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    v["access_token"].as_str().expect("token").to_string()
}

/// Every queue row with its status and last error: the message of every assertion that follows a drain.
async fn queue_dump(h: &Harness) -> String {
    let mut admin = h.ctx.state.db.admin_tx().await.expect("tx");
    let rows: Vec<(String, String, Option<String>)> =
        sqlx::query_as("SELECT kind, status::text, last_error FROM jobs ORDER BY created_at")
            .fetch_all(&mut *admin)
            .await
            .expect("jobs");
    admin.commit().await.expect("commit");
    rows.iter()
        .map(|(k, s, e)| {
            format!(
                "{k}={s}{}",
                e.as_deref().map(|e| format!(" ({e})")).unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

async fn drain_all(h: &Harness) -> usize {
    let mut total = 0;
    for _ in 0..20 {
        let n = drain(&h.ctx, "test-worker", 100).await.expect("drain");
        total += n;
        if n == 0 {
            break;
        }
    }
    total
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn generation_pipeline_produces_reviewable_reels(pool: PgPool) {
    let h = harness(pool).await;
    let noura = login(&h, "+966500000001").await;
    let (_, topics) = call(&h, Method::GET, "/admin/topics", Some(&noura), None).await;
    let topic_id = topics[0]["id"].as_str().expect("topic").to_string();

    let (s, source) = call(&h, Method::POST, "/admin/sources", Some(&noura), Some(json!({ "kind": "topic_prompt", "title": "Refunds without a receipt", "prompt": "How to handle a refund when the customer has no receipt: check the card, call a supervisor, a refund is not a loss.", "language": "en" }))).await;
    assert_eq!(s, StatusCode::CREATED, "{source}");
    let source_id = source["id"].as_str().expect("source").to_string();
    assert!(drain_all(&h).await >= 1, "extraction job ran");
    let (_, source) = call(
        &h,
        Method::GET,
        &format!("/admin/sources/{source_id}"),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(source["extraction_status"], "done", "{source}");

    let req = json!({ "source_id": source_id, "topic_id": topic_id, "languages": ["en", "ar"], "reel_count": 2, "style": "ai_avatar", "presenter": { "name": "Maha" } });
    let (s, quote) = call(
        &h,
        Method::POST,
        "/admin/generation-jobs/quote",
        Some(&noura),
        Some(req.clone()),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{quote}");
    assert!(quote["cost_credits"].as_i64().unwrap_or(0) > 0);
    let (s, job) = call(
        &h,
        Method::POST,
        "/admin/generation-jobs",
        Some(&noura),
        Some(req),
    )
    .await;
    assert_eq!(s, StatusCode::ACCEPTED, "{job}");
    let job_id = job["id"].as_str().expect("job").to_string();
    assert_eq!(
        job["steps"].as_array().map(|a| a.len()),
        Some(7),
        "extract, script, question, render, voice, caption, translate"
    );

    let ran = drain_all(&h).await;
    assert!(
        ran >= 7,
        "every step ran as its own durable job (ran {ran}); queue: {}",
        queue_dump(&h).await
    );

    let (_, job) = call(
        &h,
        Method::GET,
        &format!("/admin/generation-jobs/{job_id}"),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(job["status"], "done", "{job}");
    assert_eq!(job["progress_pct"], 100);
    assert!(
        job["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .all(|s| s["status"] == "done"),
        "{job}"
    );

    let (s, board) = call(
        &h,
        Method::GET,
        &format!("/admin/generation-jobs/{job_id}/storyboard"),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let reels = board.as_array().expect("reels");
    assert_eq!(reels.len(), 2);
    for r in reels {
        assert_eq!(r["status"], "draft", "reels wait for review");
        assert!(r["script"].as_str().map(|s| s.len() > 40).unwrap_or(false));
        assert!(r["question"].is_object(), "a recall question per reel: {r}");
        assert!(r["video_url"].is_string(), "rendered video key: {r}");
        assert!(r["captions_url"].is_string(), "captions: {r}");
        let langs: Vec<_> = r["variants"]
            .as_array()
            .expect("variants")
            .iter()
            .map(|v| v["language"].as_str().unwrap_or(""))
            .collect();
        assert!(langs.contains(&"ar"), "Arabic variant produced: {langs:?}");
    }
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn podcast_is_transcribed_and_cut(pool: PgPool) {
    let h = harness(pool).await;
    let noura = login(&h, "+966500000001").await;
    let (_, topics) = call(&h, Method::GET, "/admin/topics", Some(&noura), None).await;
    let topic_id = topics[0]["id"].as_str().expect("topic").to_string();
    let (s, slot) = call(
        &h,
        Method::POST,
        "/admin/uploads",
        Some(&noura),
        Some(json!({ "filename": "townhall.mp3", "mime": "audio/mpeg", "bytes": 1024 })),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{slot}");
    let (s, source) = call(&h, Method::POST, "/admin/sources", Some(&noura), Some(json!({ "kind": "podcast_audio", "title": "Town hall, October", "media_asset_id": slot["id"] }))).await;
    assert_eq!(s, StatusCode::CREATED, "{source}");
    let source_id = source["id"].as_str().expect("source").to_string();
    drain_all(&h).await;

    let (s, moments) = call(
        &h,
        Method::GET,
        &format!("/admin/sources/{source_id}/moments"),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let moments = moments.as_array().expect("moments");
    assert!(moments.len() >= 2, "AI-found moments: {moments:?}");
    let moment_id = moments[0]["id"].as_str().expect("moment");

    let (s, reel) = call(
        &h,
        Method::POST,
        &format!("/admin/sources/{source_id}/moments/{moment_id}/cut"),
        Some(&noura),
        Some(json!({ "topic_id": topic_id, "speaker_consent": true })),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{reel}");
    let reel_id = reel["id"].as_str().expect("reel").to_string();
    drain_all(&h).await;
    let (_, reel) = call(
        &h,
        Method::GET,
        &format!("/admin/reels/{reel_id}"),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(reel["kind"], "podcast_cut");
    assert!(reel["video_url"].is_string(), "cut rendered: {reel}");
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn daily_plans_are_built_and_whatsapp_scheduled(pool: PgPool) {
    let h = harness(pool).await;
    let tenant_id = {
        let mut admin = h.ctx.state.db.admin_tx().await.expect("tx");
        let t = reelearn_db::repos::tenants::find_by_slug(&mut admin, "alwadi")
            .await
            .expect("q")
            .expect("tenant");
        admin.commit().await.expect("commit");
        t.id
    };
    let day = reelearn_db::settings::local_today("Asia/Riyadh", chrono::Utc::now());
    {
        let mut admin = h.ctx.state.db.admin_tx().await.expect("tx");
        jobs::enqueue(
            &mut admin,
            Some(tenant_id),
            "build_daily_plans",
            json!({ "tenant_id": tenant_id, "day": day }),
            None,
        )
        .await
        .expect("enqueue");
        admin.commit().await.expect("commit");
    }
    assert!(drain_all(&h).await >= 1);

    let mut tx = h.ctx.state.db.tenant_tx(tenant_id).await.expect("tx");
    let learners = users::plan_recipients(tx.conn()).await.expect("learners");
    assert!(learners.len() >= 4);
    for u in &learners {
        let plan = plans::find_for_date(tx.conn(), u.id, day).await.expect("q");
        assert!(plan.is_some(), "{} has a plan", u.full_name);
    }
    let rizwan = learners
        .iter()
        .find(|u| u.full_name == "Rizwan Ahmed")
        .expect("rizwan");
    let plan = plans::find_for_date(tx.conn(), rizwan.id, day)
        .await
        .expect("q")
        .expect("plan");
    let scheduled = deliveries::exists_for_plan(tx.conn(), plan.id, "daily_minute")
        .await
        .expect("q");
    assert_eq!(
        scheduled,
        plan.item_count > 0,
        "WhatsApp learners get a scheduled delivery when they have items"
    );
    tx.commit().await.expect("commit");
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn nudge_is_sent_over_whatsapp(pool: PgPool) {
    let h = harness(pool).await;
    let khalid = login(&h, "+966500000002").await;
    let (_, team) = call(&h, Method::GET, "/manager/team", Some(&khalid), None).await;
    // Rizwan is the WhatsApp learner; Sara's nudge would be an app push (recorded, not sent).
    let rizwan = team["members"]
        .as_array()
        .expect("members")
        .iter()
        .find(|m| m["full_name"] == "Rizwan Ahmed")
        .map(|m| m["user_id"].clone())
        .expect("rizwan");
    let (s, v) = call(
        &h,
        Method::POST,
        "/manager/nudges",
        Some(&khalid),
        Some(json!({ "user_ids": [rizwan] })),
    )
    .await;
    assert_eq!(s, StatusCode::ACCEPTED, "{v}");
    assert!(drain_all(&h).await >= 1);
    let sent = h.whatsapp.sent.lock().expect("lock").clone();
    assert_eq!(
        sent.len(),
        1,
        "one WhatsApp message: {sent:?}; queue: {}",
        queue_dump(&h).await
    );
    assert_eq!(sent[0].0, "+966500000011");
    assert!(!sent[0].1.to_lowercase().contains("scroll"), "voice rule");
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn hrdf_export_builds_a_csv(pool: PgPool) {
    let h = harness(pool).await;
    let noura = login(&h, "+966500000001").await;
    let (s, export) = call(&h, Method::POST, "/admin/exports", Some(&noura), Some(json!({ "kind": "hrdf_evidence", "period_start": "2026-10-01", "period_end": "2026-12-31" }))).await;
    assert_eq!(s, StatusCode::ACCEPTED, "{export}");
    let id = export["id"].as_str().expect("export").to_string();
    assert!(drain_all(&h).await >= 1);
    let (_, list) = call(&h, Method::GET, "/admin/exports", Some(&noura), None).await;
    let row = list
        .as_array()
        .expect("exports")
        .iter()
        .find(|e| e["id"] == id)
        .expect("export row")
        .clone();
    assert_eq!(
        row["status"],
        "done",
        "export finished (object storage must be running): {row}; queue: {}",
        queue_dump(&h).await
    );
    let (s, dl) = call(
        &h,
        Method::GET,
        &format!("/admin/exports/{id}/download"),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{dl}");
    assert!(dl["url"]
        .as_str()
        .map(|u| u.contains(".csv"))
        .unwrap_or(false));
}
