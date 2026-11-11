//! TypeSafe Jev end to end, in production mode (each company's keys resolved per job): the
//! platform operator sets a Jev key pointing at a local mock of api.typesafe.ai, the studio
//! drafts a reel and Jev checks it, the hourly engine job asks Jev how relevant reels are to
//! each job title, and every call is metered against the platform's key at Jev's price.

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Method, Request, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use http_body_util::BodyExt;
use reelearn_api::seed::seed_demo;
use reelearn_api::{build_router, AppState, Config};
use reelearn_worker::{drain, Ctx};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

#[derive(Default)]
struct Seen {
    bearer: Vec<String>,
    question_names: Vec<String>,
    models: Vec<String>,
}

#[derive(Clone)]
struct Mock {
    seen: Arc<Mutex<Seen>>,
}

/// Answers every question with its own type: yes for statements (no for unsafe content),
/// "clear" for scores. Reports 1,000 input tokens per request.
async fn systemone(
    State(m): State<Mock>,
    headers: axum::http::HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let mut seen = m.seen.lock().unwrap();
    seen.bearer.push(
        headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string(),
    );
    seen.models
        .push(body["model"].as_str().unwrap_or("").to_string());
    let mut answers = serde_json::Map::new();
    for (name, q) in body["questions"].as_object().cloned().unwrap_or_default() {
        seen.question_names.push(name.clone());
        let a = match q["type"].as_str() {
            Some("score") => json!({ "type": "score", "score": 3.0, "confidence": 0.8 }),
            _ => {
                json!({ "type": "noul", "noul": if name == "unsafe_content" { 0.02 } else { 0.9 } })
            }
        };
        answers.insert(name, a);
    }
    (
        StatusCode::OK,
        Json(
            json!({ "model": "jev-1.14.0", "answers": answers, "usage": { "input_tokens": 1000, "output_tokens": 7 } }),
        ),
    )
}

struct Harness {
    router: Router,
    ctx: Ctx,
    mock: Mock,
    base: String,
}

async fn harness(pool: PgPool) -> Harness {
    let prefix = format!("test:{}:", uuid::Uuid::new_v4().simple());
    let state = AppState::for_tests(Config::for_tests("postgres://unused"), pool, &prefix)
        .await
        .expect("state");
    seed_demo(&state).await.expect("seed");
    let mock = Mock {
        seen: Arc::new(Mutex::new(Seen::default())),
    };
    let app = Router::new()
        .route("/v1/systemone", post(systemone))
        .route("/v1/models", get(|| async { Json(json!({ "data": [] })) }))
        .with_state(mock.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Harness {
        router: build_router(state.clone()),
        ctx: Ctx::per_company(state),
        mock,
        base,
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
    (
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        },
    )
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
    let (_, v) = call(h, Method::POST, "/auth/otp/verify", None, Some(json!({ "challenge_id": challenge, "code": "1234", "device": { "device_id": "t", "platform": "web" } }))).await;
    v["access_token"].as_str().expect("token").to_string()
}

async fn drain_all(h: &Harness) {
    for _ in 0..20 {
        if drain(&h.ctx, "jev-test", 100).await.expect("drain") == 0 {
            break;
        }
    }
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn jev_checks_drafts_matches_roles_and_is_metered_on_the_platform_key(pool: PgPool) {
    let h = harness(pool).await;
    let noura = login(&h, "+966500000001").await;
    let (s, v) = call(
        &h,
        Method::PUT,
        "/platform/integrations/ai_typesafe",
        Some(&noura),
        Some(json!({ "api_key": "ts-test-key", "base_url": h.base, "model": "jev-latest" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");

    // The studio drafts one reel; the question step hands it to Jev.
    let (_, topics) = call(&h, Method::GET, "/admin/topics", Some(&noura), None).await;
    let topic_id = topics[0]["id"].as_str().expect("topic").to_string();
    let (s, source) = call(&h, Method::POST, "/admin/sources", Some(&noura), Some(json!({ "kind": "topic_prompt", "title": "Greeting at the door", "prompt": "Greet within ten seconds; use the name twice.", "language": "en" }))).await;
    assert_eq!(s, StatusCode::CREATED, "{source}");
    drain_all(&h).await;
    let req = json!({ "source_id": source["id"], "topic_id": topic_id, "languages": ["en"], "reel_count": 1, "style": "ai_avatar", "presenter": { "name": "Maha" } });
    let (s, job) = call(
        &h,
        Method::POST,
        "/admin/generation-jobs",
        Some(&noura),
        Some(req),
    )
    .await;
    assert_eq!(s, StatusCode::ACCEPTED, "{job}");
    drain_all(&h).await;
    let job_id = job["id"].as_str().expect("job").to_string();
    let (_, job) = call(
        &h,
        Method::GET,
        &format!("/admin/generation-jobs/{job_id}"),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(job["status"], "done", "{job}");

    let reel_id: uuid::Uuid = {
        let mut admin = h.ctx.state.db.admin_tx().await.expect("tx");
        let id = sqlx::query_scalar("SELECT id FROM reels WHERE generation_job_id = $1::uuid")
            .bind(&job_id)
            .fetch_one(&mut *admin)
            .await
            .expect("a drafted reel");
        admin.commit().await.expect("commit");
        id
    };
    let (_, reel) = call(
        &h,
        Method::GET,
        &format!("/admin/reels/{reel_id}"),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(reel["quality"]["status"], "checked", "{reel}");
    assert_eq!(reel["quality"]["judge"], "typesafe-jev");
    let score = reel["quality"]["score"].as_i64().expect("score");
    assert!(score >= 70, "a clean draft scores well: {score}");
    assert_eq!(reel["quality"]["needs_attention"], false);
    {
        let seen = h.mock.seen.lock().unwrap();
        assert!(seen.bearer.iter().all(|b| b == "Bearer ts-test-key"));
        assert!(seen.models.iter().all(|m| m == "jev-latest"));
        for q in [
            "grounded",
            "unsafe_content",
            "key_correct",
            "single_answer",
            "clarity",
            "hook",
        ] {
            assert!(
                seen.question_names.iter().any(|n| n == q),
                "Jev was not asked {q}"
            );
        }
    }

    // Job-title relevance: forget the seeded judgements, then run the hourly engine job.
    let tid: uuid::Uuid = {
        let mut admin = h.ctx.state.db.admin_tx().await.expect("tx");
        sqlx::query("DELETE FROM reel_role_fit")
            .execute(&mut *admin)
            .await
            .expect("delete");
        let tid: uuid::Uuid = sqlx::query_scalar("SELECT id FROM tenants WHERE slug = 'alwadi'")
            .fetch_one(&mut *admin)
            .await
            .expect("tenant");
        reelearn_db::repos::jobs::enqueue(
            &mut admin,
            Some(tid),
            "engine_hourly",
            json!({ "tenant_id": tid, "hour": 0 }),
            None,
        )
        .await
        .expect("enqueue");
        admin.commit().await.expect("commit");
        tid
    };
    drain_all(&h).await;
    let mut tx = h.ctx.state.db.tenant_tx(tid).await.expect("tx");
    let fits: Vec<(String, f32)> = sqlx::query_as("SELECT role_key, fit FROM reel_role_fit")
        .fetch_all(tx.conn())
        .await
        .expect("fits");
    assert!(!fits.is_empty(), "role matching wrote nothing");
    assert!(fits.iter().any(|(r, _)| r == "shift supervisor"));
    assert!(fits.iter().all(|(_, f)| (*f - 0.9).abs() < 1e-6));

    // Every Jev call is metered on the platform key at $0.042 per million input tokens.
    let rows: Vec<(String, String, String, i64, Option<f64>)> = sqlx::query_as(
        "SELECT provider, capability, key_source, input_tokens, cost_usd::float8 FROM ai_usage
         WHERE provider = 'ai_typesafe' AND job_id IS NOT NULL",
    )
    .fetch_all(tx.conn())
    .await
    .expect("usage");
    tx.commit().await.expect("commit");
    assert!(rows.len() >= 2, "a draft check and role matching: {rows:?}");
    for (_, capability, source, input, cost) in &rows {
        assert_eq!(capability, "judge");
        assert_eq!(source, "platform");
        assert_eq!(*input, 1000);
        assert!((cost.expect("priced") - 0.000042).abs() < 1e-12);
    }
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn a_company_out_of_budget_still_gets_its_draft_with_a_note(pool: PgPool) {
    let h = harness(pool).await;
    let noura = login(&h, "+966500000001").await;
    call(
        &h,
        Method::PUT,
        "/platform/integrations/ai_typesafe",
        Some(&noura),
        Some(json!({ "api_key": "ts-test-key", "base_url": h.base })),
    )
    .await;
    let (_, t) = call(&h, Method::GET, "/platform/tenants", Some(&noura), None).await;
    let tid = t["tenants"][0]["id"].as_str().expect("tenant").to_string();
    let (s, _) = call(
        &h,
        Method::PUT,
        &format!("/platform/tenants/{tid}/ai-plan"),
        Some(&noura),
        Some(json!({ "platform_keys": true, "monthly_budget_usd": 0.0001 })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (_, topics) = call(&h, Method::GET, "/admin/topics", Some(&noura), None).await;
    let (_, source) = call(&h, Method::POST, "/admin/sources", Some(&noura), Some(json!({ "kind": "topic_prompt", "title": "Exits", "prompt": "Keep exits clear.", "language": "en" }))).await;
    drain_all(&h).await;
    let req = json!({ "source_id": source["id"], "topic_id": topics[0]["id"], "languages": ["en"], "reel_count": 1, "style": "ai_avatar", "presenter": { "name": "Maha" } });
    let (_, job) = call(
        &h,
        Method::POST,
        "/admin/generation-jobs",
        Some(&noura),
        Some(req),
    )
    .await;
    drain_all(&h).await;
    let (_, job) = call(
        &h,
        Method::GET,
        &format!("/admin/generation-jobs/{}", job["id"].as_str().unwrap()),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(job["status"], "done", "the judge is advisory: {job}");
    assert!(
        h.mock.seen.lock().unwrap().bearer.is_empty(),
        "no call on a spent budget"
    );
}
