//! The Meta Model API adapters against a local mock of api.meta.ai: schema-constrained chat
//! completions, image generation and diarized transcription, driven through the real pipeline.

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Method, Request, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use http_body_util::BodyExt;
use reelearn_api::seed::seed_demo;
use reelearn_api::{build_router, AppState, Config};
use reelearn_worker::providers::muse::ffmpeg;
use reelearn_worker::providers::Providers;
use reelearn_worker::{drain, Ctx};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

/// What the mock saw: every chat completion's schema name, image prompts, transcription requests.
#[derive(Default)]
struct Seen {
    schemas: Vec<String>,
    image_prompts: Vec<String>,
    transcriptions: usize,
    bearer: Vec<String>,
}

#[derive(Clone)]
struct Mock {
    seen: Arc<Mutex<Seen>>,
    image_b64: String,
}

fn answer_for(schema: &str, user_text: &str) -> Value {
    match schema {
        "extraction" => {
            json!({ "text": format!("Extracted from: {user_text}. Greet within ten seconds. Say the name once at hello and once at goodbye."), "language": "en", "key_ideas": ["Greet within ten seconds", "Use the name twice"], "suggested_reels": [{ "title": "Ten seconds", "hook": "The clock starts at the door" }] })
        }
        "reels" => json!({ "reels": [
            { "title": "Greet within ten seconds", "script": "Hi, I'm Maha. When a customer walks in, the clock starts. Ten seconds to look up, smile and say hello. That is the whole rule, and it is the one customers remember most.", "caption_line": "Ten seconds to say hello", "source_citation": "Service standard" },
            { "title": "Use the name twice", "script": "Hi, I'm Maha. Say the customer's name once at hello and once at goodbye. Twice is enough; more than that sounds like a script, and nobody likes a script.", "caption_line": "Once at hello, once at goodbye", "source_citation": "Service standard" }
        ] }),
        "question" => {
            json!({ "prompt": "A customer walks in. When do you greet them?", "options": [{ "key": "A", "text": "When they reach the till" }, { "key": "B", "text": "Within ten seconds" }, { "key": "C", "text": "When they ask for help" }], "correct_key": "B", "explanation": "The first ten seconds set the whole visit." })
        }
        "translation" => {
            json!({ "script": "[ar] script", "question": { "prompt": "متى تُلقي التحية؟", "options": [{ "key": "A", "text": "عند الصندوق" }, { "key": "B", "text": "خلال عشر ثوانٍ" }, { "key": "C", "text": "عندما يطلب المساعدة" }], "correct_key": "B", "explanation": "الثواني العشر الأولى تصنع الزيارة." } })
        }
        "moments" => {
            json!({ "moments": [{ "start_ms": 1520, "end_ms": 24640, "title": "Say the name twice", "excerpt": "Say the name twice. Once at hello, once at goodbye.", "speaker": "B", "score": 0.9 }] })
        }
        other => json!({ "error": format!("unknown schema {other}") }),
    }
}

async fn chat(
    State(m): State<Mock>,
    headers: axum::http::HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    let schema = body["response_format"]["json_schema"]["name"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let user_text = body["messages"][1]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .chars()
        .take(40)
        .collect::<String>();
    if let Ok(mut s) = m.seen.lock() {
        s.schemas.push(schema.clone());
        s.bearer.push(
            headers
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string(),
        );
    }
    assert_eq!(body["model"], "muse-spark-1.3");
    assert_eq!(body["messages"][0]["role"], "developer");
    Json(
        json!({ "id": "chatcmpl-1", "choices": [{ "message": { "role": "assistant", "content": answer_for(&schema, &user_text).to_string() } }], "usage": { "prompt_tokens": 10, "completion_tokens": 20 } }),
    )
}

async fn images(State(m): State<Mock>, Json(body): Json<Value>) -> Json<Value> {
    assert_eq!(body["model"], "muse-image-1.0");
    assert_eq!(body["size"], "1080x1920");
    if let Ok(mut s) = m.seen.lock() {
        s.image_prompts
            .push(body["prompt"].as_str().unwrap_or("").to_string());
    }
    Json(json!({ "created": 1, "data": [{ "b64_json": m.image_b64 }], "output_format": "jpeg" }))
}

async fn transcribe(State(m): State<Mock>, mut multipart: axum::extract::Multipart) -> Json<Value> {
    let mut saw_audio = false;
    let mut mode = String::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name().unwrap_or("") {
            "request" => {
                let v: Value = serde_json::from_slice(&field.bytes().await.unwrap_or_default())
                    .unwrap_or_default();
                mode = v["mode"].as_str().unwrap_or("").to_string();
            }
            "audio" => {
                saw_audio = field
                    .bytes()
                    .await
                    .map(|b| b.starts_with(b"RIFF"))
                    .unwrap_or(false)
            }
            _ => {}
        }
    }
    assert!(saw_audio, "WAV audio part present");
    assert_eq!(mode, "DIARIZATION");
    if let Ok(mut s) = m.seen.lock() {
        s.transcriptions += 1;
    }
    Json(
        json!({ "sessionId": "s1", "transcript": "Say the name twice. Once at hello, once at goodbye.", "audioDurationMs": 30000, "turns": [
        { "turnId": 1, "startMs": 1520, "endMs": 12640, "transcript": "Say the name twice.", "speaker": "A" },
        { "turnId": 2, "startMs": 13000, "endMs": 24640, "transcript": "Once at hello, once at goodbye.", "speaker": "B" }
    ] }),
    )
}

/// A tiny WAV (mono, 16-bit, 24 kHz, 0.2 s of silence) so the mock receives real RIFF bytes.
fn tiny_wav() -> Vec<u8> {
    let samples: u32 = 4800;
    let data_len = samples * 2;
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&24000u32.to_le_bytes());
    v.extend_from_slice(&48000u32.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    v.resize(v.len() + data_len as usize, 0);
    v
}

/// A keyframe for the mock image endpoint: rendered by ffmpeg when present, else a 1×1 PNG.
async fn keyframe_bytes() -> Vec<u8> {
    if ffmpeg::available().await {
        let out = std::env::temp_dir().join(format!(
            "reelearn-test-{}.jpg",
            uuid::Uuid::new_v4().simple()
        ));
        let ok = tokio::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=0x0D2B3E:s=540x960",
                "-frames:v",
                "1",
            ])
            .arg(&out)
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            if let Ok(b) = tokio::fs::read(&out).await {
                let _ = tokio::fs::remove_file(&out).await;
                return b;
            }
        }
    }
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==").unwrap_or_default()
}

struct Harness {
    router: Router,
    ctx: Ctx,
    mock: Mock,
}

async fn harness(pool: PgPool) -> Harness {
    let prefix = format!("muse:{}:", uuid::Uuid::new_v4().simple());
    let state = AppState::for_tests(Config::for_tests("postgres://unused"), pool, &prefix)
        .await
        .expect("state");
    seed_demo(&state).await.expect("seed");
    let _ = state.storage.ensure_bucket(&state.http).await;

    let mock = Mock {
        seen: Arc::new(Mutex::new(Seen::default())),
        image_b64: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            keyframe_bytes().await,
        ),
    };
    let app = Router::new()
        .route("/v1/chat/completions", post(chat))
        .route("/v1/images/generations", post(images))
        .route("/v1/asr/transcribe", post(transcribe))
        .route("/v1/models", get(|| async { Json(json!({ "data": [] })) }))
        .with_state(mock.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let base = format!("http://{}/v1", listener.local_addr().expect("addr"));
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let providers = Providers::muse(state.http.clone(), "test-key", &base);
    Harness {
        router: build_router(state.clone()),
        ctx: Ctx::new(state, providers),
        mock,
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

async fn drain_all(h: &Harness) -> usize {
    let mut total = 0;
    for _ in 0..20 {
        let n = drain(&h.ctx, "muse-test", 100).await.expect("drain");
        total += n;
        if n == 0 {
            break;
        }
    }
    total
}

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

#[sqlx::test(migrations = "../../../db/migrations")]
async fn muse_spark_and_muse_image_run_the_studio_pipeline(pool: PgPool) {
    let h = harness(pool).await;
    let noura = login(&h, "+966500000001").await;
    let (_, topics) = call(&h, Method::GET, "/admin/topics", Some(&noura), None).await;
    let topic_id = topics[0]["id"].as_str().expect("topic").to_string();

    let (s, source) = call(&h, Method::POST, "/admin/sources", Some(&noura), Some(json!({ "kind": "topic_prompt", "title": "Greeting at the door", "prompt": "Greet within ten seconds; use the name twice.", "language": "en" }))).await;
    assert_eq!(s, StatusCode::CREATED, "{source}");
    let source_id = source["id"].as_str().expect("source").to_string();
    drain_all(&h).await;
    let (_, source) = call(
        &h,
        Method::GET,
        &format!("/admin/sources/{source_id}"),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(
        source["extraction_status"],
        "done",
        "{source}; queue: {}",
        queue_dump(&h).await
    );

    let req = json!({ "source_id": source_id, "topic_id": topic_id, "languages": ["en", "ar"], "reel_count": 2, "style": "ai_avatar", "presenter": { "name": "Maha" } });
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
    drain_all(&h).await;

    let (_, job) = call(
        &h,
        Method::GET,
        &format!("/admin/generation-jobs/{job_id}"),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(
        job["status"],
        "done",
        "{job}; queue: {}",
        queue_dump(&h).await
    );
    let providers: Vec<_> = job["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| s["provider"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(providers.iter().any(|p| p == "muse-spark"), "{providers:?}");
    assert!(
        providers.iter().any(|p| p == "muse-image+ffmpeg"),
        "{providers:?}"
    );

    let (_, board) = call(
        &h,
        Method::GET,
        &format!("/admin/generation-jobs/{job_id}/storyboard"),
        Some(&noura),
        None,
    )
    .await;
    let reels = board.as_array().expect("reels");
    assert_eq!(reels.len(), 2);
    for r in reels {
        assert_eq!(r["question"]["correct_key"], "B", "{r}");
        assert!(r["video_url"].is_string(), "video stored: {r}");
        assert!(
            r["thumbnail_url"].is_string(),
            "Muse Image keyframe stored as the poster: {r}"
        );
        let langs: Vec<_> = r["variants"]
            .as_array()
            .expect("variants")
            .iter()
            .map(|v| v["language"].as_str().unwrap_or(""))
            .collect();
        assert!(langs.contains(&"ar"));
    }

    let seen = h.mock.seen.lock().expect("lock");
    assert!(
        seen.schemas.iter().any(|s| s == "extraction")
            && seen.schemas.iter().any(|s| s == "reels")
            && seen.schemas.iter().any(|s| s == "question")
            && seen.schemas.iter().any(|s| s == "translation"),
        "{:?}",
        seen.schemas
    );
    assert_eq!(seen.image_prompts.len(), 2, "one keyframe per reel");
    assert!(seen
        .image_prompts
        .iter()
        .all(|p| p.contains("9:16") && p.contains("Maha")));
    assert!(seen.bearer.iter().all(|b| b == "Bearer test-key"));
}

#[sqlx::test(migrations = "../../../db/migrations")]
async fn muse_voice_transcribe_finds_moments(pool: PgPool) {
    let h = harness(pool).await;
    let noura = login(&h, "+966500000001").await;
    let (s, slot) = call(
        &h,
        Method::POST,
        "/admin/uploads",
        Some(&noura),
        Some(json!({ "filename": "townhall.wav", "mime": "audio/wav", "bytes": 9644 })),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{slot}");
    // Put real WAV bytes where the pre-signed slot points, as the browser would.
    let put = h
        .ctx
        .state
        .http
        .put(slot["upload_url"].as_str().expect("url"))
        .header("content-type", "audio/wav")
        .body(tiny_wav())
        .send()
        .await
        .expect("upload");
    assert!(
        put.status().is_success(),
        "object storage must be running: {}",
        put.status()
    );

    let (s, source) = call(&h, Method::POST, "/admin/sources", Some(&noura), Some(json!({ "kind": "podcast_audio", "title": "Town hall", "media_asset_id": slot["id"], "language": "en" }))).await;
    assert_eq!(s, StatusCode::CREATED, "{source}");
    let source_id = source["id"].as_str().expect("source").to_string();
    drain_all(&h).await;

    let (_, source) = call(
        &h,
        Method::GET,
        &format!("/admin/sources/{source_id}"),
        Some(&noura),
        None,
    )
    .await;
    assert_eq!(
        source["extraction_status"],
        "done",
        "{source}; queue: {}",
        queue_dump(&h).await
    );
    let (_, moments) = call(
        &h,
        Method::GET,
        &format!("/admin/sources/{source_id}/moments"),
        Some(&noura),
        None,
    )
    .await;
    let moments = moments.as_array().expect("moments");
    assert_eq!(moments.len(), 1, "{moments:?}");
    assert_eq!(moments[0]["speaker"], "B");
    assert_eq!(h.mock.seen.lock().expect("lock").transcriptions, 1);
}
