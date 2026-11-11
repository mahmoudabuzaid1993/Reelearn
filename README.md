# Reelearn

A minute a day. Remembered for months.

Reelearn is a B2B platform that feeds employees short vertical learning reels on the topics their employer chooses, asks one recall question per reel, and re-asks it on a spaced schedule so the knowledge sticks. Saudi-first: Arabic and expat languages, shift-aware delivery, prayer and Ramadan aware, data kept in-Kingdom, HRDF evidence exports.

## Learning engine

Which reel comes next is decided by a ranking engine modelled on short-video recommenders, with a value model that also rewards learning. Each company sets its intensity from Gentle to Maximum in the console under **Learning engine**. Design and guardrails: [docs/engine.md](docs/engine.md); the research behind it: [docs/research/short-video-recommenders.md](docs/research/short-video-recommenders.md).

## AI providers

Meta Muse and Claude write; TypeSafe Jev checks every draft (grounded, safe, clear, correct answer key), connects tags and matches reels to job titles. Every provider is configured in the console: each company can bring its own keys (Settings → AI), otherwise it uses the platform's keys within the AI plan and monthly budget the platform operator sets (Platform). Every call is metered with whose key paid, and the month's invoice lines export as CSV. Details: [docs/ai-providers.md](docs/ai-providers.md).

## Repository layout (three tiers + mobile)

```
reelearn/
├── api/        Rust (axum + sqlx + redis)      → REST API, background worker, scheduler
├── db/         PostgreSQL migrations           → schema, indexes, partitions, RLS
├── web/        Next.js 15 (App Router, TS)     → admin console, AI Reel Studio, reports
├── mobile/     Flutter                         → employee app (swipe stack), manager app
├── openapi/    openapi.yaml                    → the contract every tier is built against
├── docs/       architecture, DB analysis, UX flows, ADRs
└── docker-compose.yml   postgres 17, redis 7, RustFS (S3 API), api, worker, web
```

Each tier only talks to the one below it: `web`/`mobile` → `api` (HTTPS/JSON) → `postgres` + `redis` + object storage. Nothing in the clients touches the database.

## Quick start

```bash
# infrastructure
docker compose up -d postgres redis minio

# API (http://localhost:8080, Swagger UI at /docs) and the worker
cd api && cp .env.example .env
cargo run -p reelearn-api -- seed      # migrations + demo tenant
cargo run -p reelearn-api              # terminal 1
cargo run -p reelearn-worker           # terminal 2: job queue, scheduler, AI pipeline (fake providers in development)

# web (http://localhost:3000)
cd web && pnpm install && pnpm dev

# mobile
cd mobile && flutter pub get && flutter run
```

Seed data (a demo tenant "Al Wadi Retail" with an admin, a manager, learners, topics and reels) is loaded by `cargo run -p reelearn-api -- seed`.

## Tests

| Tier   | Command                         | What runs                                            |
|--------|---------------------------------|------------------------------------------------------|
| db+api | `cd api && cargo test --workspace` | 58 unit tests, 12 HTTP integration tests, 5 worker end-to-end tests against Postgres + Redis + RustFS |
| web    | `cd web && pnpm test && pnpm e2e` | 43 vitest tests, 13 Playwright scenarios (mock API) |
| mobile | `cd mobile && flutter test`     | 52 widget and unit tests incl. the swipe stack and recall flow |

AI providers: set `MODEL_API_KEY` (Meta Model API: Muse Spark, Muse Image, Muse Voice Transcribe) and `PROVIDERS=real` in `api/.env`; see `docs/hackathon.md` for the Meta Global AI Developer Hackathon write-up and demo script.

Each tier keeps a `STATUS.md` with what is verified and what is not. See `docs/` for the architecture, the database analysis with every index justified, and the UX flow analysis that the screens are built from.
