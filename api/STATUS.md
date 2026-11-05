# API + worker status

Last verified 2026-11-04 on this machine with Postgres 17, Redis 7 and RustFS running from `docker-compose.yml`.

| Check | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | 52 unit tests + 12 HTTP integration tests + 6 worker unit tests + 5 worker end-to-end tests, all passing |
| Contract coverage | the `every_contract_path_is_routed` test walks every path in `openapi/openapi.yaml` and fails if one is unmounted |

## Endpoints (63 contract paths, all implemented; none stubbed)

Legend: **T** = covered by an integration test, **U** = covered by unit tests of its logic, — = implemented, exercised manually through Swagger.

| Area | Paths | Status |
|---|---|---|
| system | `/health`, `/docs`, `/openapi.yaml` | T |
| auth | `/auth/tenant/lookup`, `/auth/otp/request`, `/auth/otp/verify`, `/auth/refresh`, `/auth/logout` | T (sign-in, wrong code, lockout, rate limit) |
| me | `/me` GET/PATCH, `/me/privacy`, `/me/export` | T / — / — / T (worker) |
| learn | `/me/today`, `/me/watch`, `/me/answers`, `/me/progress`, `/me/library`, `/me/saved/{id}`, `/me/reel-requests`, `/reels/{id}` | T (plan, idempotency, SM-2 scheduling, cache invalidation, progress); library/saved/requests — |
| manager | `/manager/team`, `/manager/nudges`, `/manager/reel-requests` | T (privacy, once-per-day nudge, WhatsApp send) |
| sme | `/sme/recordings`, `/sme/recordings/{id}/submit` | — |
| tenants | `/tenants`, `/admin/tenant` GET/PATCH, `/admin/dashboard` | T (signup + isolation) / — / — |
| people | `/admin/users` GET/POST, `/admin/users/import`, `/admin/users/{id}` GET/PATCH, `/admin/groups` GET/POST, `/admin/groups/{id}/members` | T (pagination) / — |
| topics | `/admin/topics` GET/POST, `/admin/topics/{id}`, `/admin/topics/{id}/assignments` GET/PUT | — |
| studio | `/admin/uploads`, `/admin/sources` GET/POST, `/admin/sources/{id}`, `/admin/sources/{id}/moments`, `…/moments/{id}/cut`, `/admin/generation-jobs/quote`, `/admin/generation-jobs` GET/POST, `/admin/generation-jobs/{id}`, `…/events` (SSE), `…/retry`, `…/storyboard` | T (worker: extraction, quote, 7-step pipeline to storyboard, podcast transcription, moments, cut) |
| reels | `/admin/reels`, `/admin/reels/{id}` GET/PATCH, `…/regenerate`, `…/variants`, `…/submit-review`, `…/approvals`, `…/publish`, `…/archive` | T (dual approval gating, publish) |
| reports | `/admin/reports/overview`, `/admin/reports/by-topic`, `/admin/reports/by-group`, `/admin/exports` GET/POST, `/admin/exports/{id}/download` | T (HRDF export job to CSV + signed download) |
| settings | `/admin/integrations`, `/admin/integrations/{kind}` PUT, `…/test`, `/admin/audit` | — |
| webhooks | `/webhooks/whatsapp`, `/webhooks/providers/{provider}` | U (HMAC verification, command parsing); callback job path — |

## Worker

| Job kind | Status |
|---|---|
| `extract_source`, `script_reels`, `write_question`, `render_reel`, `voice_reel`, `caption_reel`, `translate_reel` | T end to end with fake providers |
| `transcribe_podcast`, `find_moments`, `cut_moment` | T |
| Meta Model API adapters (Muse Spark, Muse Image + ffmpeg, Muse Voice Transcribe) | T against a local mock of api.meta.ai (`tests/muse.rs`): schema names, bearer auth, keyframes, WAV diarization, posters stored |
| `provider_callback` (async vendors) | implemented; needs a real vendor to exercise |
| `build_daily_plans` + WhatsApp scheduling | T |
| `send_delivery` (otp, invite, review request, daily minute, nudge, bot reply) | T (nudge over WhatsApp); others — |
| `hrdf_export`, `data_export` | T / — |
| `maintain_partitions`, scheduler leader lease, stale-job release, backoff | U (backoff); scheduler tick — |

## Known limits

- App push and e-mail deliveries are recorded as sent-by-log (no push/e-mail provider wired yet).
- Meta Model API has no text-to-speech or video generation: narration stays on the voice provider, video is keyframe + ffmpeg.
- Dubbing reuses the master video with translated captions until a dub vendor is configured.
- Real video/voice/ASR vendors are adapters with a generic job contract; the exact field mapping per vendor lives in `crates/worker/src/providers/real.rs`.
- `GET /admin/generation-jobs/{id}/events` streams snapshots by polling the database every 2 s; a LISTEN/NOTIFY push can replace that without a contract change.
