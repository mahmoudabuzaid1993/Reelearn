# Reelearn architecture

## 1. Shape

Three tiers, strictly layered. Clients never see the database.

```
┌──────────────────────┐   ┌──────────────────────┐
│  web  (Next.js 15)   │   │  mobile (Flutter)    │
│  admin · studio ·    │   │  employee · manager  │
│  reports · manager   │   │                      │
└──────────┬───────────┘   └──────────┬───────────┘
           │ HTTPS JSON (OpenAPI 3.1) │
           └────────────┬─────────────┘
                        ▼
┌───────────────────────────────────────────────────┐
│  api (Rust, axum)  ── stateless, N replicas        │
│  ├─ reelearn-api     HTTP, auth, rate limits       │
│  ├─ reelearn-worker  generation pipeline, deliveries│
│  └─ reelearn-scheduler  daily plans, spaced review │
└──────┬──────────────┬───────────────┬─────────────┘
       ▼              ▼               ▼
  PostgreSQL 17    Redis 7        Object storage (S3 API)
  system of record cache, locks,  reels, captions, uploads
  + job queue      rate limits    served through a CDN
```

External providers are called only from the worker, never from a request handler: Claude (extraction, scripts, questions, translation), Higgsfield (avatar and b-roll video), Veo (motion scenes), a voice provider (narration and dubbing), an ASR provider (podcast transcripts), WhatsApp Business API, Microsoft Entra (SSO), HRIS connectors (Jisr, Bayzat).

## 2. Tenancy

- One database, one schema; every domain row carries `tenant_id`.
- Every tenant-scoped index leads with `tenant_id`, so each tenant's working set is contiguous on disk and lookups never scan other tenants.
- Postgres row-level security is enabled on every tenant table. The API sets `SET LOCAL app.tenant_id = '<uuid>'` at the start of each transaction from the verified JWT. A query that forgets a `WHERE tenant_id` still cannot leak across tenants.
- Tenants carry a `region` (`ksa`, `uae`, `eu`). Deployments are per region: a region's API only mounts tenants of that region, which is how "data stays in the Kingdom" is enforced, not by policy text.

## 3. Request path and caching

Read-mostly, bursty: every learner in a tenant opens the app at shift start.

| Layer | What | TTL / invalidation |
|---|---|---|
| CDN | reel video, captions, thumbnails (immutable, content-addressed keys) | 1 year, keys never reused |
| HTTP | `ETag` + `Cache-Control: private, max-age=0, must-revalidate` on GET lists and reel metadata | revalidate; 304 is cheap |
| Redis `plan:{tenant}:{user}:{date}` | today's plan, fully materialised JSON | expires at end of the learner's day; deleted on watch/answer |
| Redis `reel:{id}:{lang}` | published reel metadata + question (no media URLs) | 10 min; deleted on publish/update/archive |
| Redis `media:{key}` | signed object URL | 50 min (URLs are signed for 60) |
| Redis `dash:{tenant}:{range}` and `report:{tenant}:{kind}:{range}` | dashboard and report aggregates | 60 s |
| Redis `rl:{scope}:{id}` | token-bucket rate limits (OTP, writes per user, per tenant) | window |
| Redis `idem:{tenant}:{key}` | idempotency keys for POST watch/answer (phones retry on bad networks) | 24 h |
| Redis `revoked:{jti}` | access-token revocation (logout, deactivation) | token lifetime |
| Postgres | `user_stats`, `daily_topic_stats` are maintained incrementally; reports never `COUNT(*)` over event tables | on write / nightly |

Cache is a hint, never truth: every cached object is rebuilt from Postgres on miss, and a Redis outage degrades to slower, not wrong.

## 4. Background work

A Postgres-backed job queue (`jobs` table, `FOR UPDATE SKIP LOCKED`) rather than Redis streams: a job must survive a cache restart, and the queue has to be transactional with the rows it changes (a generation step and its reel update commit together). Redis would be faster but is the wrong durability class.

Job kinds: `extract_source`, `script_reels`, `render_reel`, `caption_reel`, `write_question`, `translate_reel`, `transcribe_podcast`, `find_moments`, `cut_moment`, `build_daily_plans`, `send_delivery`, `refresh_stats`, `hrdf_export`, `data_export`.

- Workers are idempotent (every job carries an `attempt` and a deterministic output key).
- Exponential backoff with jitter, dead-letter after 8 attempts, visible in the admin UI as a failed step.
- The scheduler is a single leader (Redis `SET NX` lease, 30 s) that enqueues per-tenant `build_daily_plans` jobs at the tenant's configured hour; the plans themselves are built by any worker.

## 5. Daily plan and the recall engine

The "minute" is computed, not browsed:

1. Due reviews: `memory_states` rows for this user with `due_on <= today`, ordered by `due_on`, capped at the tenant's `max_reviews_per_day` (default 1).
2. New reels: published reels from the user's topic assignments the user has not seen, respecting `reels_per_day` on the assignment (default 2) and the assignment's channel.
3. Items are frozen into `daily_plan_items` so the plan is stable for the day and identical on app and WhatsApp.

Spaced repetition is SM-2 with fixed first intervals (1, 3, 10, 30 days) because learners see each question rarely; the ease factor only moves after the second repetition. A wrong answer re-schedules at 3 days and never shows red: the client shows the right answer in mint and moves on.

What managers and HR can see is a data rule, not a UI rule: endpoints under `/manager` and `/admin/reports` read from aggregate tables that carry no `question_id`, `chosen_key` or timestamps finer than a day. Per-answer rows are reachable only through `/me`.

## 5a. The learning engine

Practice ("keep going" after the minute) and the order of new reels in the daily plan come from the learning engine. It works like a short-video recommender: signals, smoothed predictions, a weighted value model with negatives subtracted, then re-ranking for colleague shares, exploration, topic diversity and reviews. Each company tunes its intensity from Gentle to Maximum, within guardrails that stay on. See [engine.md](engine.md).

## 6. Scalability notes

- API is stateless; scale horizontally behind a load balancer. One replica handles roughly 2–3k req/s of cached reads on 2 vCPU.
- Postgres: PgBouncer in transaction mode in front of the primary; one read replica for reports and exports (`DATABASE_READ_URL`). Event tables (`watch_events`, `answers`, `audit_log`, `deliveries`) are range-partitioned by month with BRIN indexes on time so inserts stay append-only and old partitions are detached, not deleted row by row.
- Media never passes through the API: uploads go to object storage with pre-signed PUT URLs; downloads are pre-signed GET URLs behind a CDN.
- WhatsApp fan-out at shift start is rate-limited per provider account and spread across the delivery window by the scheduler, not sent in one burst.
- Generation is expensive and slow (minutes). It is queued, priced in credits up front (`generation_jobs.cost_credits`), and the storyboard is reviewable before any render spend.

## 7. Security

- Learners authenticate by phone + WhatsApp OTP (no passwords); admins by OTP or Entra SSO. Access tokens: 15 min JWT (RS256, tenant_id + role claims); refresh tokens: opaque, hashed, rotated on every use, bound to a device id.
- OTP codes are argon2-hashed, 5 attempts, 5 minutes, 3 requests per phone per hour.
- All writes are audited (`audit_log`) with actor, entity and a JSON diff.
- Provider secrets live in `integrations.config` encrypted with a per-deployment key (AES-GCM), never logged.
- Problem+JSON (RFC 9457) errors; no stack traces leave the process; request ids on every response.

## 8. Clean-code conventions

- Rust: `api/crates/{domain,db,api,worker}`. `domain` has no I/O; `db` has the only SQL; `api` maps HTTP ↔ domain; `worker` runs jobs. Errors are typed (`thiserror`), handlers return `Result<Json<T>, ApiError>`. No `unwrap` outside tests.
- Web: feature folders under `web/src/features/<area>`; server components fetch, client components interact; API calls only through the generated typed client; forms validated with zod; no business rules in components.
- Mobile: `mobile/lib/features/<area>/{data,domain,presentation}`; Riverpod for state; the swipe stack is a pure widget tested with widget tests; all HTTP through one Dio client with the generated models.
- Every tier has lint on CI (`clippy -D warnings`, `eslint` + `tsc --noEmit`, `flutter analyze`) and tests that must pass before merge.
