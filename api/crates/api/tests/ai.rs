//! AI providers over HTTP: the platform area (operators only), platform keys, company AI
//! plans and budgets, which key a company resolves to, routing, usage and the invoice export.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use reelearn_api::seed::seed_demo;
use reelearn_api::{build_router, AppState, Config};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

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

async fn put(app: &App, path: &str, token: &str, body: Value) -> (StatusCode, Value) {
    let (s, v, _) = call(app, Method::PUT, path, Some(token), Some(body), &[]).await;
    (s, v)
}

const ADMIN: &str = "+966500000001";
const MANAGER: &str = "+966500000002";

fn provider<'a>(overview: &'a Value, kind: &str) -> &'a Value {
    overview["providers"]
        .as_array()
        .expect("providers")
        .iter()
        .find(|p| p["kind"] == kind)
        .expect("provider row")
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn platform_area_is_for_operators_only(pool: PgPool) {
    let app = app(pool).await;
    let manager = login(&app, MANAGER).await;
    let (s, v) = get(&app, "/platform/me", &manager).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["operator"], false);
    for path in [
        "/platform/integrations",
        "/platform/tenants",
        "/platform/usage.csv",
    ] {
        let (s, _) = get(&app, path, &manager).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "{path}");
    }
    let admin = login(&app, ADMIN).await;
    let (_, v) = get(&app, "/platform/me", &admin).await;
    assert_eq!(v["operator"], true, "the demo admin runs the platform");
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn company_key_wins_then_platform_key_within_plan_and_budget(pool: PgPool) {
    let app = app(pool).await;
    let admin = login(&app, ADMIN).await;

    // No keys anywhere: Jev is not configured for the company.
    let (s, v) = get(&app, "/admin/ai", &admin).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(provider(&v, "ai_typesafe")["unavailable"], "not_configured");

    // The platform sets a key: the company now uses it, and the key never comes back.
    let (s, v) = put(
        &app,
        "/platform/integrations/ai_typesafe",
        &admin,
        json!({ "api_key": "ts-platform-secret", "model": "jev-1.14.0" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let row = v
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "ai_typesafe")
        .unwrap();
    assert_eq!(row["status"], "connected");
    assert_eq!(row["source"], "console");
    assert_eq!(row["public_config"]["api_key"], "••••");
    assert_eq!(row["public_config"]["model"], "jev-1.14.0");
    assert!(!v.to_string().contains("ts-platform-secret"));
    let (_, v) = get(&app, "/admin/ai", &admin).await;
    let p = provider(&v, "ai_typesafe");
    assert_eq!(p["using"], "platform");
    assert_eq!(p["platform_key"], true);
    assert_eq!(p["model"], "jev-1.14.0");

    // Unknown settings are refused rather than stored.
    let (s, _) = put(
        &app,
        "/admin/integrations/ai_typesafe",
        &admin,
        json!({ "webhook": "x" }),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);

    // The plan stops including platform keys: unavailable until the company brings its own.
    let (_, t) = get(&app, "/platform/tenants", &admin).await;
    let tenant_id = t["tenants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["slug"] == "alwadi")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (s, v) = put(
        &app,
        &format!("/platform/tenants/{tenant_id}/ai-plan"),
        &admin,
        json!({ "platform_keys": false, "markup_pct": 20 }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let (_, v) = get(&app, "/admin/ai", &admin).await;
    assert_eq!(provider(&v, "ai_typesafe")["unavailable"], "not_in_plan");
    assert_eq!(v["plan"]["platform_keys"], false);

    // The company's own key always wins.
    let (s, _) = put(
        &app,
        "/admin/integrations/ai_typesafe",
        &admin,
        json!({ "api_key": "ts-company-secret" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, v) = get(&app, "/admin/ai", &admin).await;
    let p = provider(&v, "ai_typesafe");
    assert_eq!(p["using"], "company");
    assert_eq!(p["company_key"], true);

    // Back on platform keys with a budget the seeded month already spent: budget reached.
    let (s, _) = put(
        &app,
        "/admin/integrations/ai_typesafe",
        &admin,
        json!({ "api_key": null }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, v) = put(
        &app,
        &format!("/platform/tenants/{tenant_id}/ai-plan"),
        &admin,
        json!({ "platform_keys": true, "monthly_budget_usd": 0.0001, "markup_pct": 0 }),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let (_, v) = get(&app, "/admin/ai", &admin).await;
    assert_eq!(provider(&v, "ai_typesafe")["unavailable"], "budget_reached");

    // Invalid plans are refused.
    let (s, _) = put(
        &app,
        &format!("/platform/tenants/{tenant_id}/ai-plan"),
        &admin,
        json!({ "markup_pct": 9000 }),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    let (s, _) = put(
        &app,
        &format!("/platform/tenants/{tenant_id}/ai-plan"),
        &admin,
        json!({ "allowed": ["ai_nope"] }),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn routing_usage_and_invoice_export(pool: PgPool) {
    let app = app(pool).await;
    let admin = login(&app, ADMIN).await;

    let (s, v) = put(&app, "/admin/ai", &admin, json!({ "writer": "claude", "quality_check": true, "review_threshold": 80, "role_matching": false, "tag_check": true })).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["routing"]["writer"], "claude");
    assert_eq!(v["routing"]["review_threshold"], 80);
    let (s, _) = put(
        &app,
        "/admin/ai",
        &admin,
        json!({ "review_threshold": 101 }),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);

    // The seed meters two weeks of this month: platform Muse and Jev, company Claude.
    let (s, u) = get(&app, "/admin/ai/usage", &admin).await;
    assert_eq!(s, StatusCode::OK, "{u}");
    let lines = u["lines"].as_array().unwrap();
    assert!(lines
        .iter()
        .any(|l| l["provider"] == "ai_typesafe" && l["key_source"] == "platform"));
    assert!(lines
        .iter()
        .any(|l| l["provider"] == "ai_claude" && l["key_source"] == "company"));
    let platform_cost: f64 = lines
        .iter()
        .filter(|l| l["key_source"] == "platform")
        .filter_map(|l| l["cost_usd"].as_f64())
        .sum();
    // The seeded plan adds 20 % on platform-key usage.
    let billable = u["platform_billable_usd"].as_f64().unwrap();
    assert!(
        (billable - platform_cost * 1.2).abs() < 1e-6,
        "{billable} vs {platform_cost}"
    );
    let (s, _) = get(&app, "/admin/ai/usage?month=2026-13", &admin).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);

    let (s, t) = get(&app, "/platform/tenants", &admin).await;
    assert_eq!(s, StatusCode::OK);
    let row = t["tenants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["slug"] == "alwadi")
        .unwrap();
    assert!((row["billable_usd"].as_f64().unwrap() - billable).abs() < 1e-6);
    assert!(row["budget_used"].as_f64().unwrap() > 0.0);

    let (s, csv, headers) = call(
        &app,
        Method::GET,
        "/platform/usage.csv",
        Some(&admin),
        None,
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(headers[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .starts_with("text/csv"));
    let text = csv.as_str().unwrap();
    assert!(text.starts_with("company,provider,capability,key_source"));
    assert!(text.contains("alwadi,ai_typesafe,judge,platform,jev-1.14.0"));
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn reviewers_see_the_quality_check(pool: PgPool) {
    let app = app(pool).await;
    let admin = login(&app, ADMIN).await;
    let (s, v) = get(&app, "/admin/reels?status=in_review", &admin).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let items = v["items"].as_array().unwrap();
    assert!(
        items.iter().any(|r| r["quality"]["score"].is_number()),
        "{v}"
    );
}
