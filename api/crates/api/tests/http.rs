//! HTTP integration tests: the router against a fresh Postgres database per test
//! (`#[sqlx::test]` applies `db/migrations`), Redis on `REDIS_URL`, and the demo seed.

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
    state: AppState,
}

async fn app(pool: PgPool) -> App {
    let prefix = format!("test:{}:", uuid::Uuid::new_v4().simple());
    let state = AppState::for_tests(Config::for_tests("postgres://unused"), pool, &prefix)
        .await
        .expect("state");
    seed_demo(&state).await.expect("seed");
    App {
        router: build_router(state.clone()),
        state,
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
const KHALID: &str = "+966500000002";
const HANA: &str = "+966500000003";
const NOURA: &str = "+966500000001";

#[sqlx::test(migrations = "../../../db/migrations")]
async fn health_reports_dependencies(pool: PgPool) {
    let _ = MIGRATIONS;
    let app = app(pool).await;
    let (s, v, _) = call(&app, Method::GET, "/health", None, None, &[]).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["postgres"], true);
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn otp_sign_in_returns_profile_with_tenant(pool: PgPool) {
    let app = app(pool).await;
    let token = login(&app, SARA).await;
    let (s, me) = get(&app, "/me", &token).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(me["tenant"]["slug"], "alwadi");
    assert_eq!(me["role"], "learner");
    assert_eq!(me["first_name"], "Sara");
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn wrong_code_then_lockout(pool: PgPool) {
    let app = app(pool).await;
    let (_, v) = post(
        &app,
        "/auth/otp/request",
        None,
        json!({ "tenant_slug": "alwadi", "phone": SARA }),
    )
    .await;
    let challenge = v["challenge_id"].as_str().expect("challenge").to_string();
    let mut last = StatusCode::OK;
    for _ in 0..6 {
        let (s, _) = post(&app, "/auth/otp/verify", None, json!({ "challenge_id": challenge, "code": "0000", "device": { "device_id": "d", "platform": "ios" } })).await;
        last = s;
    }
    assert_eq!(
        last,
        StatusCode::LOCKED,
        "after too many wrong codes the challenge locks"
    );
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn otp_requests_are_rate_limited(pool: PgPool) {
    let app = app(pool).await;
    if !app.state.cache.is_connected() {
        eprintln!("redis not available; skipping rate-limit assertion");
        return;
    }
    let mut statuses = Vec::new();
    for _ in 0..4 {
        let (s, _) = post(
            &app,
            "/auth/otp/request",
            None,
            json!({ "tenant_slug": "alwadi", "phone": "+966500000013" }),
        )
        .await;
        statuses.push(s);
    }
    assert_eq!(statuses[3], StatusCode::TOO_MANY_REQUESTS, "{statuses:?}");
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn today_answer_idempotency_and_recall(pool: PgPool) {
    let app = app(pool).await;
    let token = login(&app, SARA).await;
    let (s, plan) = get(&app, "/me/today", &token).await;
    assert_eq!(s, StatusCode::OK, "{plan}");
    let items = plan["items"].as_array().expect("items");
    assert!(!items.is_empty(), "the seeded learner has a plan");
    // Learner payloads never carry the answer key.
    assert!(
        !plan.to_string().contains("correct_key"),
        "correct_key leaked into the plan"
    );

    let item = items
        .iter()
        .find(|i| i["question"].is_object())
        .expect("an item with a question");
    let qid = item["question"]["id"].as_str().expect("question id");
    let body = json!({ "plan_item_id": item["id"], "question_id": qid, "chosen_key": "B", "response_ms": 2100 });
    let (s, first, _) = call(
        &app,
        Method::POST,
        "/me/answers",
        Some(&token),
        Some(body.clone()),
        &[("idempotency-key", "k1")],
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{first}");
    assert!(first["correct_key"].is_string());
    assert!(
        first["next_due_on"].as_str().expect("due") > plan["plan_date"].as_str().expect("date"),
        "spaced repetition schedules the question in the future"
    );

    let (s, replay, headers) = call(
        &app,
        Method::POST,
        "/me/answers",
        Some(&token),
        Some(body),
        &[("idempotency-key", "k1")],
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(replay, first, "same key replays the same outcome");
    assert!(headers.get("idempotent-replayed").is_some());

    let (s, _) = post(
        &app,
        "/me/answers",
        Some(&token),
        json!({ "plan_item_id": item["id"], "question_id": qid, "chosen_key": "A" }),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT, "a plan item is answered once");

    let (_, after) = get(&app, "/me/today", &token).await;
    let state = after["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|i| i["id"] == item["id"])
        .map(|i| i["state"].clone());
    assert_eq!(
        state,
        Some(json!("answered")),
        "the cached plan was invalidated"
    );
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn watch_marks_item_and_progress_reads(pool: PgPool) {
    let app = app(pool).await;
    let token = login(&app, SARA).await;
    let (_, plan) = get(&app, "/me/today", &token).await;
    let reel_item = plan["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|i| i["reel"].is_object())
        .expect("reel item")
        .clone();
    let body = json!({ "plan_item_id": reel_item["id"], "reel_id": reel_item["reel"]["id"], "watched_ms": 44000, "completed": true });
    let (s, _, _) = call(
        &app,
        Method::POST,
        "/me/watch",
        Some(&token),
        Some(body),
        &[("idempotency-key", "w1")],
    )
    .await;
    assert_eq!(s, StatusCode::ACCEPTED);
    let (s, progress) = get(&app, "/me/progress", &token).await;
    assert_eq!(s, StatusCode::OK);
    assert!(progress["remembered_pct"].is_number());
    assert!(progress["stats"]["streak_current"].is_number());
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn tenant_isolation_across_workspaces(pool: PgPool) {
    let app = app(pool).await;
    let noura = login(&app, NOURA).await;
    let (s, reels) = get(&app, "/admin/reels?limit=5", &noura).await;
    assert_eq!(s, StatusCode::OK);
    let reel_id = reels["items"][0]["id"]
        .as_str()
        .expect("a reel")
        .to_string();

    let (s, created) = post(&app, "/tenants", None, json!({ "company_name": "Other Co", "slug": "otherco", "admin_name": "Omar", "admin_email": "omar@otherco.example", "admin_phone": "+966511111111", "default_language": "en", "region": "ksa" })).await;
    assert_eq!(s, StatusCode::CREATED, "{created}");
    let challenge = created["challenge_id"].as_str().expect("challenge");
    let (s, tokens) = post(&app, "/auth/otp/verify", None, json!({ "challenge_id": challenge, "code": "1234", "device": { "device_id": "web", "platform": "web" } })).await;
    assert_eq!(s, StatusCode::OK, "{tokens}");
    let omar = tokens["access_token"].as_str().expect("token");

    let (s, list) = get(&app, "/admin/reels", omar).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        list["items"].as_array().map(|a| a.len()),
        Some(0),
        "a new tenant sees no reels of another tenant"
    );
    let (s, _) = get(&app, &format!("/admin/reels/{reel_id}"), omar).await;
    assert_eq!(
        s,
        StatusCode::NOT_FOUND,
        "row-level security hides the other tenant's reel"
    );
    let (s, users) = get(&app, "/admin/users", omar).await;
    assert_eq!(s, StatusCode::OK);
    assert!(users["items"]
        .as_array()
        .expect("items")
        .iter()
        .all(|u| u["full_name"] != "Sara Al-Qahtani"));
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn publish_needs_both_approvals(pool: PgPool) {
    let app = app(pool).await;
    let noura = login(&app, NOURA).await;
    let hana = login(&app, HANA).await;
    let (_, groups) = get(&app, "/admin/groups", &noura).await;
    let group_id = groups[0]["id"].as_str().expect("group").to_string();
    let (_, list) = get(&app, "/admin/reels?status=in_review", &noura).await;
    let reel_id = list["items"][0]["id"]
        .as_str()
        .expect("an in_review reel")
        .to_string();

    let (s, v) = post(
        &app,
        &format!("/admin/reels/{reel_id}/publish"),
        Some(&noura),
        json!({ "group_ids": [group_id] }),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT, "no approvals yet: {v}");

    let (s, _) = post(
        &app,
        &format!("/admin/reels/{reel_id}/approvals"),
        Some(&noura),
        json!({ "decision": "approved", "comment": "L&D ok" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, v) = post(
        &app,
        &format!("/admin/reels/{reel_id}/publish"),
        Some(&noura),
        json!({ "group_ids": [group_id] }),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT, "L&D alone is not enough: {v}");

    let (s, _) = post(
        &app,
        &format!("/admin/reels/{reel_id}/approvals"),
        Some(&hana),
        json!({ "decision": "approved" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, v) = post(
        &app,
        &format!("/admin/reels/{reel_id}/publish"),
        Some(&noura),
        json!({ "group_ids": [group_id] }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["status"], "published");
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn manager_sees_readiness_but_no_answers(pool: PgPool) {
    let app = app(pool).await;
    let khalid = login(&app, KHALID).await;
    let (s, team) = get(&app, "/manager/team", &khalid).await;
    assert_eq!(s, StatusCode::OK, "{team}");
    let members = team["members"].as_array().expect("members");
    assert!(members.iter().any(|m| m["full_name"] == "Sara Al-Qahtani"));
    let text = team.to_string();
    for forbidden in [
        "chosen_key",
        "question_id",
        "answered_at",
        "saved",
        "is_correct",
    ] {
        assert!(
            !text.contains(forbidden),
            "manager payload leaks {forbidden}"
        );
    }
    // A learner cannot open manager routes.
    let sara = login(&app, SARA).await;
    let (s, _) = get(&app, "/manager/team", &sara).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn nudges_are_once_per_day(pool: PgPool) {
    let app = app(pool).await;
    let khalid = login(&app, KHALID).await;
    let (_, team) = get(&app, "/manager/team", &khalid).await;
    let sara_id = team["members"]
        .as_array()
        .expect("members")
        .iter()
        .find(|m| m["full_name"] == "Sara Al-Qahtani")
        .map(|m| m["user_id"].clone())
        .expect("sara");
    let (s, first) = post(
        &app,
        "/manager/nudges",
        Some(&khalid),
        json!({ "user_ids": [sara_id] }),
    )
    .await;
    assert_eq!(s, StatusCode::ACCEPTED, "{first}");
    assert_eq!(first["queued"], 1);
    let (_, second) = post(
        &app,
        "/manager/nudges",
        Some(&khalid),
        json!({ "user_ids": [sara_id] }),
    )
    .await;
    assert_eq!(second["queued"], 0);
    assert_eq!(second["skipped_already_nudged"], 1);
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn cursor_pagination_walks_people(pool: PgPool) {
    let app = app(pool).await;
    let noura = login(&app, NOURA).await;
    let (s, page1) = get(&app, "/admin/users?limit=3", &noura).await;
    assert_eq!(s, StatusCode::OK, "{page1}");
    let cursor = page1["next_cursor"]
        .as_str()
        .expect("more pages")
        .to_string();
    let (s, page2) = get(
        &app,
        &format!("/admin/users?limit=3&cursor={cursor}"),
        &noura,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let ids1: Vec<_> = page1["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|u| u["id"].clone())
        .collect();
    let ids2: Vec<_> = page2["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|u| u["id"].clone())
        .collect();
    assert!(!ids2.is_empty());
    assert!(
        ids1.iter().all(|id| !ids2.contains(id)),
        "pages do not overlap"
    );
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn every_contract_path_is_routed(pool: PgPool) {
    let app = app(pool).await;
    let spec = std::fs::read_to_string("../../../openapi/openapi.yaml").expect("openapi.yaml");
    let noura = login(&app, NOURA).await;
    let mut missing = Vec::new();
    for line in spec.lines() {
        if let Some(path) = line.strip_prefix("  /") {
            let path = format!("/{}", path.trim_end_matches(':'));
            let probe = path
                .replace("{reel_id}", "00000000-0000-0000-0000-000000000000")
                .replace("{user_id}", "00000000-0000-0000-0000-000000000000")
                .replace("{topic_id}", "00000000-0000-0000-0000-000000000000")
                .replace("{source_id}", "00000000-0000-0000-0000-000000000000")
                .replace("{job_id}", "00000000-0000-0000-0000-000000000000")
                .replace("{export_id}", "00000000-0000-0000-0000-000000000000")
                .replace("{group_id}", "00000000-0000-0000-0000-000000000000")
                .replace("{moment_id}", "00000000-0000-0000-0000-000000000000")
                .replace("{recording_id}", "00000000-0000-0000-0000-000000000000")
                .replace("{kind}", "whatsapp")
                .replace("{provider}", "veo");
            let (s, _, _) = call(&app, Method::GET, &probe, Some(&noura), None, &[]).await;
            // 404 with the router's "no route" would mean the path is not mounted; a 405 or any other status means it is.
            if s == StatusCode::NOT_FOUND {
                let (s2, body, _) = call(
                    &app,
                    Method::POST,
                    &probe,
                    Some(&noura),
                    Some(json!({})),
                    &[],
                )
                .await;
                if s2 == StatusCode::NOT_FOUND && body.to_string().contains("no route") {
                    missing.push(path);
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "contract paths without a route: {missing:?}"
    );
}
