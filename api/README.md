# Reelearn API and worker (Rust)

Two binaries from one workspace, sharing the same configuration, Postgres pool, Redis cache and object storage:

- `reelearn-api` — the HTTP API (axum) behind `openapi/openapi.yaml`. Swagger UI at `/docs`.
- `reelearn-worker` — the durable job queue, the scheduler leader and the AI Reel Studio pipeline.

```
crates/domain   pure rules: enums, SM-2 recall, plan selection, streaks, pricing, cursors, phone normalisation
crates/db       sqlx repositories (the only SQL), tenant transactions with row-level security, migrations
crates/api      HTTP: auth, middleware, routes, services, seed
crates/worker   jobs, providers (fake + HTTP adapters), queue loop, scheduler
```

## Run locally

```bash
cd .. && docker compose up -d postgres redis minio     # infra (RustFS provides the S3 API on :9000)
cp .env.example .env
cargo run -p reelearn-api -- seed                      # migrations + the demo tenant "alwadi"
cargo run -p reelearn-api                               # http://localhost:8080  (Swagger: /docs)
cargo run -p reelearn-worker                            # in a second terminal
```

Development sign-in: any seeded phone with OTP code `1234` (the code is logged, never sent). Admin: `+966500000001` (Noura), manager `+966500000002` (Khalid), SME `+966500000003` (Hana), learner `+966500000010` (Sara).

## Tests

```bash
export DATABASE_URL=postgres://reelearn:reelearn@localhost:5432/reelearn REDIS_URL=redis://localhost:6379 APP_ENV=test
cargo test --workspace            # unit tests + HTTP integration tests + worker end-to-end tests
cargo clippy --all-targets -- -D warnings && cargo fmt --all --check
```

`#[sqlx::test]` creates a fresh database per test and applies `db/migrations`; Redis keys are prefixed per test. The integration tests cover OTP sign-in and lockout, OTP rate limiting, today's plan, answers with idempotency replay and spaced scheduling, cache invalidation, tenant isolation through RLS, dual approval before publishing, manager privacy, cursor pagination, every contract path being routed, and, for the worker, the full generation pipeline, podcast transcription and cuts, daily plan building with WhatsApp scheduling, nudges and the HRDF export.

## How requests are isolated

Every tenant-scoped handler opens `Db::tenant_tx(tenant_id)`, which runs `SET LOCAL ROLE reelearn_app` and `set_config('app.tenant_id', …)` inside the transaction. All tenant tables have `FORCE ROW LEVEL SECURITY` with a policy on `app.tenant_id`, so a query that forgets `WHERE tenant_id` still cannot read or write another tenant. The job queue and the scheduler use `admin_tx()` (BYPASSRLS) only to find work, then open a tenant transaction to do it.

## Caching (Redis)

| Key | Content | Lifetime |
|---|---|---|
| `plan:{tenant}:{user}:{date}` | today's plan JSON | until local midnight; deleted on watch/answer and after a plan build |
| `reel:{id}:{lang}` | playable reel metadata | 10 min; deleted on publish/update/archive |
| `media:{key}` | signed object URL | 50 min |
| `dash:{tenant}:{range}`, `report:{tenant}:{kind}:{range}` | aggregates | 60 s |
| `rl:*` | OTP and write rate limits | window |
| `idem:{tenant}:{key}` | idempotent replays of `/me/watch` and `/me/answers` | 24 h |
| `lease:scheduler` | scheduler leader lease | 60 s, renewed |

A Redis outage degrades to slower, never wrong: every cached object is rebuilt from Postgres.

## The worker

Jobs live in the `jobs` table and are claimed with `FOR UPDATE SKIP LOCKED`. Failures retry with exponential backoff and jitter (5 s → 30 min) up to `max_attempts` (8), then dead-letter with the error visible to the admin. Stuck jobs (worker died) are released after 15 minutes.

Job kinds: `extract_source`, `script_reels`, `write_question`, `render_reel`, `voice_reel`, `caption_reel`, `translate_reel`, `transcribe_podcast`, `find_moments`, `cut_moment`, `provider_callback`, `build_daily_plans`, `send_delivery`, `hrdf_export`, `data_export`, `maintain_partitions`.

The scheduler leader (Redis `SET NX` lease) enqueues `build_daily_plans` per tenant at the tenant's local `plan_build_hour`, turns due `deliveries` rows into `send_delivery` jobs, and keeps next month's partitions ready.

### Providers

The pipeline is written against traits (`ScriptWriter`, `VideoRenderer`, `VoiceSynth`, `Transcriber`, `WhatsAppSender`). In development and tests the deterministic fakes run the whole studio with no vendor account: plausible scripts and questions from the source text, tiny placeholder MP4/VTT objects, and a recorded WhatsApp outbox.

In production (`PROVIDERS=real` or `APP_ENV=production`):

- **Meta Model API** (`MODEL_API_KEY`, the default brain): Muse Spark (`muse-spark-1.3`) for extraction (PDF inline, images, URLs, briefs), scripts, questions, translation and podcast moments, always through `response_format: json_schema`; Muse Image for a 9:16 keyframe per reel, which ffmpeg turns into the reel video (and the poster); Muse Voice Transcribe for diarized podcast transcription (WAV; ffmpeg converts other formats). See `docs/hackathon.md`.
- **Claude** (`ANTHROPIC_API_KEY`, or `WRITER_PROVIDER=claude` when both keys exist): the same writer role via the Messages API.
- **Video / voice / ASR** are async job vendors behind one adapter each (`crates/worker/src/providers/real.rs`): the worker POSTs `{base}/jobs`, stores the returned reference on the step, and the vendor calls back `POST /v1/webhooks/providers/{provider}` with `{ "ref", "status": "done"|"failed", "url", "duration_ms", "error" }` (transcription adds `"segments": [{start_ms, end_ms, text, speaker}]`). Mapping a specific vendor's field names is a change to that one file.
- **WhatsApp Business Cloud API** (`WHATSAPP_PHONE_NUMBER_ID`, `WHATSAPP_ACCESS_TOKEN`) for OTPs, invites, the daily minute, nudges and bot replies. Inbound messages arrive at `POST /v1/webhooks/whatsapp` and drive the same plan as the app.

App push and email deliveries are recorded as sent-by-log until a push/email provider is wired in; the delivery rows and the audit trail are already in place.
