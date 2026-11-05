# Database analysis and schema

Postgres 17. One schema, multi-tenant by `tenant_id` on every domain table, row-level security on all of them. The DDL lives in `db/migrations/`; this document explains *why* each table and index exists and what access pattern it serves.

## 1. Access patterns that drive the design

| # | Pattern | Frequency | Shape |
|---|---|---|---|
| P1 | Learner opens the app: load today's plan | every learner, once or twice a day, bursty at shift start | point lookup by (tenant, user, date), then reels + questions by id |
| P2 | Learner watches a reel / answers a question | 3–6 writes per learner per day | append-only inserts + one upsert into `memory_states` + `user_stats` |
| P3 | Scheduler builds tomorrow's plans | once per tenant per day, all users | range scan of `memory_states` by `due_on`; set difference of published reels vs seen |
| P4 | Admin library: list reels by status/topic/language, search | continuous, small result pages | filtered, ordered pagination; full-text search |
| P5 | Studio: generation job progress | polling / SSE while a job runs | point lookups by job id; worker queue pops |
| P6 | Manager: team readiness | daily, per manager | aggregate per (user, topic), no per-answer detail |
| P7 | Reports / HRDF export | weekly, monthly | aggregates per (topic, group, day) over months |
| P8 | WhatsApp inbound bot | per message | find user by phone, find today's plan, next item |
| P9 | Delivery fan-out | per shift window | `deliveries` where scheduled and due, by time |

Design consequences:

- **Hot path P1 is a cache hit** (Redis) and, on miss, three indexed lookups. Nothing on this path aggregates.
- **Event tables are append-only and partitioned** (`watch_events`, `answers`, `deliveries`, `audit_log`): monthly range partitions, BRIN on time, no updates. Aggregates for P6/P7 come from `daily_topic_stats` and `user_topic_readiness`, which are maintained by jobs, so reports never touch the raw partitions.
- **Every composite index starts with `tenant_id`**: a tenant's rows cluster together in the index, and RLS predicates (`tenant_id = current_setting('app.tenant_id')`) are satisfied from the index itself.
- **UUIDv7 primary keys** (`uuid_generate_v7()`, time-ordered) so inserts append at the right edge of the B-tree instead of splitting random pages, and ids stay globally unique across regions.

## 2. Tables

### Identity and tenancy

- `tenants` — the company workspace. `slug` is the subdomain (`alwadi`), `region` pins residency, `settings` jsonb holds delivery windows, prayer/Ramadan calendars, `reels_per_day`, `max_reviews_per_day`, default languages.
- `users` — everyone in a tenant. `role` ∈ admin, manager, sme, learner. Phone is the learner identity (`phone_e164`), email for admins/SSO. `manager_id` self-reference powers the manager views. `delivery_channel` (app / whatsapp / both) and `delivery_slot` (shift_start / break / shift_end) are per user because the same tenant mixes office staff and warehouse staff.
- `groups`, `group_members` — teams, locations, roles. Assignments target groups, never individual users, so onboarding a person is adding them to a group.
- `otp_challenges`, `refresh_tokens`, `devices` — auth. Codes and tokens are stored hashed.

### Content

- `topics` — what the company teaches (Customer service, Store safety, AML). Carries `compliance_tag` for HRDF evidence.
- `topic_assignments` — topic × group with cadence, channel, mandatory flag and date range. This is the "Topics & channels" screen.
- `sources` — anything uploaded or linked: pdf, pptx, image, url, youtube, topic_prompt, podcast_audio, video, sme_recording. `source_extractions` keeps the extracted text separately so `sources` rows stay small.
- `source_moments` — AI-found moments in a podcast or talk (start/end ms, excerpt, score) that can be cut into reels.
- `generation_jobs`, `generation_steps` — the studio pipeline: one job per source request, one step row per pipeline stage with provider ids and logs. `cost_credits` is computed before the job starts so the admin sees the price on the storyboard.
- `reels` — the unit of learning. `kind` (ai_avatar, motion, broll, podcast_cut, sme_recorded), `status` lifecycle draft → in_review → approved → published → archived (or rejected). `script`, `source_citation`, `presenter` jsonb, media keys, `version`.
- `reel_variants` — one row per language: video, captions, audio, dub status. The reel's own `language` is the master.
- `questions` — exactly one recall question per reel per language (`UNIQUE (reel_id, language)`), options as jsonb `[{key,text}]`, `correct_key`, `explanation`.
- `approvals` — dual sign-off rows (L&D and SME) with decision and comment.

### Learning

- `daily_plans`, `daily_plan_items` — the materialised "minute" per user per day. Items reference a reel and/or question with `reason` (new / review) and a `state` that advances as the learner swipes.
- `memory_states` — spaced-repetition state per (user, question): `due_on`, `interval_days`, `ease`, `reps`, `lapses`, `last_result`. PK is `(user_id, question_id)`; the scheduler scans `(tenant_id, due_on)`.
- `watch_events` (partitioned) — every reel play with `watched_ms`, `completed`, channel, device.
- `answers` (partitioned) — every answer with `chosen_key`, `is_correct`, `response_ms`, `attempt_no`.
- `saved_reels`, `reel_requests` — the Saved tab and "ask for a reel" (requests are stored with `user_id` but surfaced to managers anonymised by the API).
- `user_stats` — streak, freezes, counters; one row per user, updated in the same transaction as the event.
- `user_topic_readiness` — (user, topic) → `ready` bool + `recall_pct`, refreshed on each answer. This is the *only* per-user learning data a manager can read.
- `daily_topic_stats` — (tenant, topic, group, day) counters for dashboards and HRDF exports.

### Delivery and operations

- `deliveries` (partitioned) — scheduled sends: channel, `scheduled_for`, `sent_at`, provider message id, status. The worker pops `status='scheduled' AND scheduled_for <= now()`.
- `whatsapp_messages` — inbound and outbound bot messages for audit and the "same reel, same question" WhatsApp-only mode.
- `jobs` — the durable queue (kind, payload, run_at, attempts, locked_by). Popped with `SKIP LOCKED`.
- `integrations` — per-tenant provider configs (encrypted blob) and status.
- `media_assets` — object-storage keys with sha256, size, mime; everything binary points here.
- `exports` — HRDF evidence packs and personal data exports (job-produced files).
- `audit_log` (partitioned) — append-only actor/action/entity/diff.

## 3. Indexes, each with its reason

Format: `table (columns) [type/partial]` — pattern it serves.

**tenants / users / groups**
- `tenants (slug)` unique — subdomain lookup on every sign-in (P8 too).
- `users (tenant_id, phone_e164)` unique where phone not null — OTP sign-in and WhatsApp inbound (P8).
- `users (tenant_id, lower(email))` unique where email not null — admin sign-in, SSO.
- `users (tenant_id, status, role)` — people list filters.
- `users (tenant_id, manager_id)` where manager_id not null — manager's team (P6).
- `users (tenant_id, employee_no)` unique where not null — HRIS sync upserts.
- `group_members (user_id)` — "which groups is this user in" for plan building (PK already covers group → users).
- `refresh_tokens (token_hash)` unique; `refresh_tokens (user_id)` — rotation and logout-all.
- `otp_challenges (phone_e164, created_at desc)` — throttle and latest challenge; rows are short-lived.

**content**
- `topics (tenant_id, slug)` unique.
- `topic_assignments (tenant_id, group_id)` and `(tenant_id, topic_id)` — both directions of the assignment matrix.
- `sources (tenant_id, created_at desc)` — studio list.
- `source_moments (source_id, score desc)` — top moments for the podcast cut screen.
- `generation_jobs (tenant_id, created_at desc)`; `generation_jobs (status) where status in ('queued','running')` — admin list and worker/ops view (P5).
- `generation_steps (job_id, position)` unique — ordered step list, one point lookup per poll.
- `reels (tenant_id, status, updated_at desc)` — library default view (P4).
- `reels (tenant_id, topic_id, published_at desc) where status = 'published'` — plan building: published reels per topic (P3); small partial index, only what learners can see.
- `reels (tenant_id, generation_job_id)` where not null — storyboard.
- `reels` GIN on `to_tsvector('simple', title || ' ' || coalesce(script,''))` — library search; `simple` config because titles mix Arabic, English and Urdu.
- `reel_variants (reel_id, language)` unique — variant pick by language.
- `questions (reel_id, language)` unique — one question per reel per language, fetched with the reel.
- `approvals (reel_id)` — sign-off panel.
- `media_assets (tenant_id, sha256)` — dedupe identical uploads.

**learning (the hot path)**
- `daily_plans (tenant_id, user_id, plan_date)` unique — P1 point lookup; also makes plan building idempotent.
- `daily_plan_items (plan_id, position)` unique — ordered items.
- `daily_plan_items (tenant_id, reel_id)` — "who has reel X planned" when a reel is archived mid-day.
- `memory_states (tenant_id, user_id, due_on)` — P3 per user (and P1 "what's due").
- `memory_states (tenant_id, due_on)` — P3 scanning a whole tenant's due reviews in one range scan.
- `answers (tenant_id, user_id, answered_at desc)` — personal progress (P2/P7 personal).
- `answers (tenant_id, question_id, answered_at)` — reel-level recall score in the library.
- `answers` BRIN `(answered_at)` per partition — monthly stats jobs scan by time cheaply.
- `watch_events (tenant_id, user_id, started_at desc)`, `(tenant_id, reel_id, started_at)`, BRIN `(started_at)` — same reasoning.
- `saved_reels` PK `(user_id, reel_id)` + `(reel_id)` — Saved tab, and cascade on archive.
- `user_topic_readiness` PK `(user_id, topic_id)` + `(tenant_id, topic_id, ready)` — manager view counts.
- `daily_topic_stats` PK `(tenant_id, topic_id, group_id, day)` + `(tenant_id, day)` — report ranges.
- `user_stats (tenant_id, last_active_on)` — "inactive 3 days" nudges.

**delivery and ops**
- `deliveries (status, scheduled_for) where status = 'scheduled'` — the fan-out pop (P9); tiny partial index over a huge table.
- `deliveries (tenant_id, user_id, scheduled_for desc)` — per-user delivery history.
- `jobs (run_at) where status = 'queued'` — queue pop with SKIP LOCKED; `jobs (kind, status)` for ops.
- `whatsapp_messages (tenant_id, phone_e164, created_at desc)` — bot conversation context.
- `audit_log (tenant_id, at desc)`, `(tenant_id, entity, entity_id)` — audit screen and "history of this reel".
- `integrations (tenant_id, kind)` unique.
- `exports (tenant_id, created_at desc)`.

## 4. What is deliberately not indexed

- `reels.script` by B-tree, `users.full_name` by B-tree: searched through the GIN / trigram index instead.
- Low-cardinality columns alone (`status`, `role`, `kind`): only useful as a trailing column after `tenant_id`, which is how they appear above.
- `created_at` on small tables: the UUIDv7 primary key is already time-ordered.

## 5. Partitioning and retention

- `watch_events`, `answers`, `deliveries`, `audit_log`: `PARTITION BY RANGE` on the time column, monthly. A `maintain_partitions` job creates next month's partition and the default partition catches stragglers.
- Retention is per tenant in `tenants.settings.retention_months` (default 36 for HRDF evidence). Expired partitions are detached and archived to object storage before drop.

## 6. Integrity rules enforced in the database

- `CHECK (jsonb_typeof(options) = 'array')` on questions; `correct_key` must be one of the option keys (trigger).
- A reel can only be `published` when it has an approved L&D and an approved SME approval (trigger `reels_require_dual_approval`).
- `memory_states.ease BETWEEN 1.3 AND 3.0`, `interval_days >= 0`.
- `daily_plan_items.kind = 'question_only'` ⇒ `question_id IS NOT NULL`.
- Foreign keys everywhere, `ON DELETE CASCADE` only from parent to pure children (plan → items, reel → variants/questions); users and reels are soft-deleted (`status`), never deleted, because events reference them.

## 7. Row-level security

`ALTER TABLE … ENABLE ROW LEVEL SECURITY` + policy `USING (tenant_id = current_setting('app.tenant_id', true)::uuid)` on every tenant table. The API role is `reelearn_app` (not superuser, not table owner), so policies apply. Migrations and the scheduler run as `reelearn_admin` with `BYPASSRLS` for cross-tenant jobs.
