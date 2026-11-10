//! Learning-engine integration tests over HTTP: practice batches, signals, topics, sharing,
//! company tuning and insights, against a fresh seeded database per test.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use reelearn_api::seed::seed_demo;
use reelearn_api::{build_router, AppState, Config};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

const MIGRATIONS: &str = "../../../db/migrations";

struct App {
    router: Router,
}

async fn app(pool: PgPool) -> App {
    let prefix = format!("test:{}:", uuid::Uuid::new_v4().simple());
    let state = AppState::for_tests(Config::for_tests("postgres://unused"), pool, &prefix)
        .await
        .expect("state");
    seed_demo(&state).await.expect("seed");
    App {
        router: build_router(state),
    }
}

async fn call(
    app: &App,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
    extra: &[(&str, &str)],
) -> (StatusCode, Value, axum::http::HeaderMap) {
    let mut req = Request::builder().method(method).uri(format!("/v1{path}"));
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    for (k, v) in extra {
        req = req.header(*k, *v);
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .expect("request");
    let resp = app.router.clone().oneshot(req).await.expect("response");
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into()))
    };
    (status, json, headers)
}

async fn get(app: &App, path: &str, token: &str) -> (StatusCode, Value) {
    let (s, v, _) = call(app, Method::GET, path, Some(token), None, &[]).await;
    (s, v)
}

async fn post(app: &App, path: &str, token: Option<&str>, body: Value) -> (StatusCode, Value) {
    let (s, v, _) = call(app, Method::POST, path, token, Some(body), &[]).await;
    (s, v)
}

/// Sign in with the development OTP (1234) and return the access token.
async fn login(app: &App, phone: &str) -> String {
    let (s, v) = post(
        app,
        "/auth/otp/request",
        None,
        json!({ "tenant_slug": "alwadi", "phone": phone }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "otp request: {v}");
    let challenge = v["challenge_id"].as_str().expect("challenge").to_string();
    let (s, v) = post(app, "/auth/otp/verify", None, json!({ "challenge_id": challenge, "code": "1234", "device": { "device_id": "test-device", "platform": "ios" } })).await;
    assert_eq!(s, StatusCode::OK, "otp verify: {v}");
    v["access_token"].as_str().expect("token").to_string()
}

const SARA: &str = "+966500000010";
const RIZWAN: &str = "+966500000011";
const NOURA: &str = "+966500000001";

async fn put(app: &App, path: &str, token: &str, body: Value) -> (StatusCode, Value) {
    let (s, v, _) = call(app, Method::PUT, path, Some(token), Some(body), &[]).await;
    (s, v)
}

/// Open practice whatever the wall clock says (tests run outside delivery windows too).
async fn open_practice(app: &App, admin: &str) {
    let (s, v) = put(
        app,
        "/admin/engine",
        admin,
        json!({ "practice_outside_work_hours": true, "exploration_rate": 0.0 }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["settings"]["intensity"], "custom");
}

fn reel_ids(batch: &Value) -> Vec<String> {
    batch["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter_map(|i| i["reel"]["id"].as_str().map(str::to_string))
        .collect()
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn practice_ranks_shares_first_never_repeats_in_a_session_and_records_impressions(
    pool: PgPool,
) {
    let _ = MIGRATIONS;
    let app = app(pool.clone()).await;
    let admin = login(&app, NOURA).await;
    open_practice(&app, &admin).await;
    let sara = login(&app, SARA).await;

    let (s, today) = get(&app, "/me/today", &sara).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(today["practice_available"], true);

    let (s, first) = get(&app, "/me/next?limit=4", &sara).await;
    assert_eq!(s, StatusCode::OK, "{first}");
    assert_eq!(first["budget"]["open"], true, "{first}");
    let items = first["items"].as_array().expect("items");
    assert!(!items.is_empty(), "{first}");
    assert_eq!(
        items[0]["reason"], "shared",
        "the seeded colleague share leads: {first}"
    );
    assert_eq!(items[0]["reason_detail"], "Fatimah");
    for item in items {
        assert!(item["reason"].is_string() && item["id"].is_string());
    }
    let session = first["session_id"].as_str().expect("session").to_string();

    let (s, second) = get(
        &app,
        &format!("/me/next?limit=4&session_id={session}"),
        &sara,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{second}");
    assert_eq!(second["session_id"], session.as_str());
    let a = reel_ids(&first);
    for id in reel_ids(&second) {
        assert!(!a.contains(&id), "reel {id} served twice in one session");
    }

    let impressions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM engagement_events WHERE session_id = $1::uuid AND kind = 'impression' AND score IS NOT NULL AND reason IS NOT NULL",
    )
    .bind(&session)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(
        impressions as usize,
        reel_ids(&first).len() + reel_ids(&second).len()
    );
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn the_budget_closes_practice_by_company_settings(pool: PgPool) {
    let app = app(pool.clone()).await;
    let admin = login(&app, NOURA).await;
    let sara = login(&app, SARA).await;

    let (s, v) = put(
        &app,
        "/admin/engine",
        &admin,
        json!({ "intensity": "gentle" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["settings"]["intensity"], "gentle");
    let (_, batch) = get(&app, "/me/next", &sara).await;
    assert_eq!(batch["budget"]["open"], false);
    assert_eq!(batch["budget"]["stop"], "disabled");
    assert_eq!(batch["items"].as_array().map(Vec::len), Some(0));
    let (_, today) = get(&app, "/me/today", &sara).await;
    assert_eq!(today["practice_available"], false);

    // A one-minute session cap: spend it, and the session closes.
    let (s, v) = put(
        &app,
        "/admin/engine",
        &admin,
        json!({ "intensity": "custom", "continuous_enabled": true, "session_minutes_cap": 1, "daily_minutes_cap": 0, "practice_outside_work_hours": true }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let (_, batch) = get(&app, "/me/next", &sara).await;
    assert_eq!(batch["budget"]["open"], true, "{batch}");
    assert_eq!(batch["budget"]["remaining_session_s"], 60);
    let session = batch["session_id"].as_str().expect("session");
    let (s, v) = post(
        &app,
        "/me/events",
        Some(&sara),
        json!({ "session_id": session, "active_ms": 61_000, "events": [] }),
    )
    .await;
    assert_eq!(s, StatusCode::ACCEPTED, "{v}");
    let (_, after) = get(&app, &format!("/me/next?session_id={session}"), &sara).await;
    assert_eq!(after["budget"]["stop"], "session_cap", "{after}");
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn signals_move_interest_and_reel_statistics(pool: PgPool) {
    let app = app(pool.clone()).await;
    let admin = login(&app, NOURA).await;
    open_practice(&app, &admin).await;
    let sara = login(&app, SARA).await;
    let (_, batch) = get(&app, "/me/next?limit=3", &sara).await;
    let reel = batch["items"][0]["reel"]["id"]
        .as_str()
        .expect("reel")
        .to_string();
    let topic_of = |pool: PgPool, reel: String| async move {
        sqlx::query_scalar::<_, uuid::Uuid>("SELECT topic_id FROM reels WHERE id = $1::uuid")
            .bind(reel)
            .fetch_one(&pool)
            .await
            .expect("topic")
    };
    let topic = topic_of(pool.clone(), reel.clone()).await;
    let interest = |v: &Value| -> i64 {
        v["topics"]
            .as_array()
            .expect("topics")
            .iter()
            .find(|t| t["topic_id"] == topic.to_string().as_str())
            .and_then(|t| t["interest"].as_i64())
            .unwrap_or(0)
    };
    let (s, before) = get(&app, "/me/topics", &sara).await;
    assert_eq!(s, StatusCode::OK, "{before}");

    let (s, v) = post(
        &app,
        "/me/events",
        Some(&sara),
        json!({ "events": [ { "reel_id": reel, "kind": "start" }, { "reel_id": reel, "kind": "rewatch", "watched_ms": 30000 }, { "reel_id": reel, "kind": "more_like_this" } ] }),
    )
    .await;
    assert_eq!(s, StatusCode::ACCEPTED, "{v}");
    assert_eq!(v["recorded"], 3);
    let (_, after) = get(&app, "/me/topics", &sara).await;
    assert!(interest(&after) > interest(&before), "{before} -> {after}");

    let (s, _) = post(&app, "/me/events", Some(&sara), json!({ "events": [ { "reel_id": reel, "kind": "not_relevant" }, { "reel_id": reel, "kind": "not_relevant" }, { "reel_id": reel, "kind": "not_relevant" } ] })).await;
    assert_eq!(s, StatusCode::ACCEPTED);
    let (_, later) = get(&app, "/me/topics", &sara).await;
    assert!(
        interest(&later) < interest(&after),
        "negative feedback lowers interest"
    );

    // Completions and answers arrive through the existing endpoints and still count.
    let stats = |pool: PgPool, reel: String| async move {
        sqlx::query_as::<_, (i32, i32, i32)>("SELECT completions, not_relevant, rewatches FROM reel_engagement_stats WHERE reel_id = $1::uuid")
            .bind(reel)
            .fetch_one(&pool)
            .await
            .expect("stats")
    };
    let (c0, nr, rw) = stats(pool.clone(), reel.clone()).await;
    assert!(nr >= 3 && rw >= 1);
    let (s, _) = post(
        &app,
        "/me/watch",
        Some(&sara),
        json!({ "reel_id": reel, "watched_ms": 29000, "completed": true }),
    )
    .await;
    assert_eq!(s, StatusCode::ACCEPTED);
    let (c1, _, _) = stats(pool.clone(), reel.clone()).await;
    assert_eq!(c1, c0 + 1);

    // Server-side kinds are refused from the app.
    let (s, v) = post(
        &app,
        "/me/events",
        Some(&sara),
        json!({ "events": [ { "reel_id": reel, "kind": "complete" } ] }),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{v}");
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn people_follow_and_mute_topics_but_not_mandatory_ones(pool: PgPool) {
    let app = app(pool.clone()).await;
    let admin = login(&app, NOURA).await;
    open_practice(&app, &admin).await;
    let sara = login(&app, SARA).await;
    let (_, topics) = get(&app, "/me/topics", &sara).await;
    let list = topics["topics"].as_array().expect("topics");
    let mandatory = list
        .iter()
        .find(|t| t["is_mandatory"] == true)
        .expect("a mandatory topic");
    let optional = list
        .iter()
        .find(|t| t["is_mandatory"] == false)
        .expect("an optional topic");

    let (s, v) = put(
        &app,
        &format!("/me/topics/{}", mandatory["topic_id"].as_str().expect("id")),
        &sara,
        json!({ "preference": "mute" }),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{v}");

    let muted = optional["topic_id"].as_str().expect("id").to_string();
    let (s, v) = put(
        &app,
        &format!("/me/topics/{muted}"),
        &sara,
        json!({ "preference": "mute" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let row = v["topics"]
        .as_array()
        .expect("t")
        .iter()
        .find(|t| t["topic_id"] == muted.as_str())
        .cloned()
        .expect("row");
    assert_eq!(row["preference"], "mute");

    let (_, batch) = get(&app, "/me/next?limit=20", &sara).await;
    for item in batch["items"].as_array().expect("items") {
        if let Some(id) = item["reel"]["id"].as_str() {
            let t: uuid::Uuid =
                sqlx::query_scalar("SELECT topic_id FROM reels WHERE id = $1::uuid")
                    .bind(id)
                    .fetch_one(&pool)
                    .await
                    .expect("topic");
            assert_ne!(t.to_string(), muted, "a muted topic was served");
        }
    }
    let (s, v) = put(
        &app,
        &format!("/me/topics/{muted}"),
        &sara,
        json!({ "preference": null }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let (s, _) = put(
        &app,
        &format!("/me/topics/{muted}"),
        &sara,
        json!({ "preference": "loud" }),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn colleagues_share_reels_and_the_share_leads_their_next_batch(pool: PgPool) {
    let app = app(pool.clone()).await;
    let admin = login(&app, NOURA).await;
    open_practice(&app, &admin).await;
    let sara = login(&app, SARA).await;
    let (s, people) = get(&app, "/me/colleagues?q=Riz", &sara).await;
    assert_eq!(s, StatusCode::OK, "{people}");
    let rizwan_id = people[0]["id"].as_str().expect("rizwan").to_string();
    assert_eq!(people[0]["full_name"], "Rizwan Ahmed");

    let (_, batch) = get(&app, "/me/next?limit=3", &sara).await;
    let reel = batch["items"][1]["reel"]["id"]
        .as_str()
        .expect("reel")
        .to_string();
    let (s, v) = post(
        &app,
        "/me/shares",
        Some(&sara),
        json!({ "reel_id": reel, "recipient_ids": [rizwan_id], "note": "Saw this today" }),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{v}");
    assert_eq!(v["sent"], 1);
    let (_, again) = post(
        &app,
        "/me/shares",
        Some(&sara),
        json!({ "reel_id": reel, "recipient_ids": [rizwan_id] }),
    )
    .await;
    assert_eq!(again["sent"], 0, "the same share is sent once");

    let rizwan = login(&app, RIZWAN).await;
    let (_, theirs) = get(&app, "/me/next?limit=3", &rizwan).await;
    assert_eq!(theirs["items"][0]["reason"], "shared", "{theirs}");
    assert_eq!(theirs["items"][0]["reel"]["id"], reel.as_str());
    assert_eq!(theirs["items"][0]["reason_detail"], "Sara");

    // Opening it marks the share opened.
    let (s, _) = post(
        &app,
        "/me/events",
        Some(&rizwan),
        json!({ "events": [ { "reel_id": reel, "kind": "start" } ] }),
    )
    .await;
    assert_eq!(s, StatusCode::ACCEPTED);
    let opened: bool = sqlx::query_scalar("SELECT opened_at IS NOT NULL FROM reel_shares WHERE reel_id = $1::uuid AND recipient_id = $2::uuid")
        .bind(&reel)
        .bind(&rizwan_id)
        .fetch_one(&pool)
        .await
        .expect("share");
    assert!(opened);

    // A company can switch sharing off.
    let (s, _) = put(
        &app,
        "/admin/engine",
        &admin,
        json!({ "peer_sharing": false }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = post(
        &app,
        "/me/shares",
        Some(&sara),
        json!({ "reel_id": reel, "recipient_ids": [rizwan_id] }),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn admins_tune_the_engine_and_read_insights(pool: PgPool) {
    let app = app(pool).await;
    let admin = login(&app, NOURA).await;
    let sara = login(&app, SARA).await;

    let (s, v) = get(&app, "/admin/engine", &admin).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["settings"]["intensity"], "balanced");
    for p in ["gentle", "balanced", "intense", "maximum"] {
        assert_eq!(v["presets"][p]["intensity"], p);
    }
    let (s, _) = get(&app, "/admin/engine", &sara).await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    let (s, v) = put(
        &app,
        "/admin/engine",
        &admin,
        json!({ "weights": { "share": 9 }, "batch_size": 1 }),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{v}");
    let (s, v) = put(
        &app,
        "/admin/engine",
        &admin,
        json!({ "intensity": "maximum" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["settings"]["daily_minutes_cap"], 0);
    assert_eq!(v["settings"]["max_nudges_per_day"], 3);
    let (s, v) = put(
        &app,
        "/admin/engine",
        &admin,
        json!({ "weights": { "learning": 2.0 } }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["settings"]["intensity"], "custom");
    assert_eq!(v["settings"]["weights"]["learning"], 2.0);

    let (s, ins) = get(&app, "/admin/engine/insights?range=30d", &admin).await;
    assert_eq!(s, StatusCode::OK, "{ins}");
    assert_eq!(ins["intensity"], "custom");
    assert!(ins["sessions"].as_i64().unwrap_or(0) > 0, "{ins}");
    assert!(ins["practice_minutes"].as_f64().unwrap_or(0.0) > 0.0);
    assert!(!ins["daily"].as_array().expect("daily").is_empty());
    assert!(!ins["topics"].as_array().expect("topics").is_empty());
    assert!(!ins["tags"].as_array().expect("tags").is_empty());
    assert!(!ins["reels"].as_array().expect("reels").is_empty());
    assert_eq!(ins["bands"].as_array().map(Vec::len), Some(5));
    assert!(ins["sharing"]["shares"].as_i64().unwrap_or(0) >= 1);
    let audit: Value = get(&app, "/admin/audit", &admin).await.1;
    assert!(audit["items"]
        .as_array()
        .expect("audit")
        .iter()
        .any(|a| a["action"] == "engine.update"));
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn a_person_can_turn_personalisation_off(pool: PgPool) {
    let app = app(pool).await;
    let admin = login(&app, NOURA).await;
    open_practice(&app, &admin).await;
    let sara = login(&app, SARA).await;
    let (_, topics) = get(&app, "/me/topics", &sara).await;
    assert_eq!(topics["personalized"], true);
    let (s, v) = put(
        &app,
        "/me/personalization",
        &sara,
        json!({ "enabled": false }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["personalized"], false);
    let (_, batch) = get(&app, "/me/next?limit=10", &sara).await;
    for item in batch["items"].as_array().expect("items") {
        let reason = item["reason"].as_str().unwrap_or("");
        assert!(
            !matches!(reason, "interest" | "popular" | "explore"),
            "profiling reason {reason} with personalisation off"
        );
    }
}
