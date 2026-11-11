# AI providers

Every AI and messaging provider Reelearn uses is configured from the console, per company, with a
platform layer above it for commercials. Code: `api/crates/domain/src/ai.rs` (rules, no I/O),
`api/crates/api/src/services/ai.rs` (resolution), `api/crates/api/src/routes/platform.rs`,
`api/crates/worker/src/providers/` (adapters), migration `db/migrations/0005_ai_providers.sql`.

## Who does what

| Provider | Kind | Does |
|---|---|---|
| Meta Muse | `ai_muse` | Writes scripts, questions, translations and podcast moments; transcribes; makes 9:16 keyframes (ffmpeg turns them into video) |
| Claude | `ai_claude` | Writes the same things as Muse; the company picks which one writes |
| TypeSafe Jev | `ai_typesafe` | **Judges, never writes.** Checks each draft, checks and connects tags, matches reels to job titles |
| Video vendor | `ai_higgsfield` | Avatar and motion video (async job API, callback webhook) |
| Voice vendor | `voice` | Narration and dubbing |
| Transcription vendor | `asr` | Transcripts (Muse transcribes when this is not set) |
| WhatsApp Business | `whatsapp` | Daily minute and reminders |

The company admin chooses the writer (automatic: Muse when it has a key, else Claude) and switches
the Jev features under **Settings → AI**.

## Whose key is used

For each provider, for each company, on every job (`services::ai::resolve`):

1. **The company's own key** (Settings → Integrations, or "Add your key" on the AI tab). Always wins.
2. **The platform's key**, set by the platform operator under **Platform → Platform keys** (or, as a
   fallback, in the server environment), **if** the company's AI plan allows platform keys for that
   provider **and** this month's billed platform-key spend is under the plan's budget.
3. Otherwise the provider is unavailable. A writer kept off by the plan or the budget fails the step
   with the reason shown to the admin ("this month's AI budget on the platform's key is used up; add
   your own key or raise the plan") instead of producing placeholder content. Jev is advisory: when it
   is unavailable the draft is still made and carries a note.

Development (`PROVIDERS` unset or `fake`) runs every company on deterministic fakes, including a fake
judge, so the whole flow works offline.

## The AI plan (commercials)

Set per company by the platform operator (`PUT /platform/tenants/{id}/ai-plan`):

- `platform_keys`: may the company use Reelearn's keys at all ("bring your own keys" tier when off)
- `allowed`: which providers are included on Reelearn's keys (empty = all), e.g. include Jev but not Claude
- `monthly_budget_usd`: ceiling on billed platform-key spend per calendar month (null = unlimited)
- `markup_pct`: added to platform-key cost on the company's bill (0 to 500)

Every billable call lands in `ai_usage` (monthly partitions) with provider, capability (write,
judge, render, voice, transcribe), **key source** (company or platform), model, tokens, units and a
cost from the provider's prices. Prices are entered per provider by the platform operator (per
million input and output tokens, or per call); TypeSafe's published Jev price ($0.042 per million
input tokens, output free) is built in. An unpriced call is counted, never costed as zero.

The platform operator downloads the month's invoice lines as CSV (`GET /platform/usage.csv`):
one line per company × provider × capability × key source × model with cost, markup and billable
amount. Companies see their own usage under Settings → AI.

Platform operators are people in `platform_operators`, not a company role. Grant with:

```
reelearn-api grant-platform-operator someone@example.com
```

The check runs on every request, so removing the row revokes access at once. Platform tables are
revoked from the tenant-scoped database role.

## TypeSafe Jev

Jev (TypeSafe AI's first System One model) answers typed questions about a state in one parallel
pass and returns calibrated probabilities: Noul (probability a statement is true), Score (expected
level on a rubric) and Choice. It does not generate text and cannot return a malformed answer.
`POST https://api.typesafe.ai/v1/systemone`, bearer key, `model: jev-latest` by default.

Following TypeSafe's guidance, **Jev makes the judgements and code makes the decisions**. Every
question's wording lives in `providers/jev.rs`; every rule that turns answers into a decision lives
in `reelearn_domain::ai::review`, unit-tested.

### Draft quality check (after the question step)

State: the reel (title, topic, language, script), up to 12,000 characters of the source, the
question. Questions:

| Name | Type | Asks |
|---|---|---|
| `grounded` | noul | Every factual claim in the script is supported by the source |
| `on_topic` | noul | The script teaches something about the reel's topic |
| `unsafe_content` | noul | The script contains content unsuitable for training at work (hazard training excluded) |
| `caption_fits` | noul | The caption says something the script says (when there is one) |
| `clarity` | score 0–4 | How easy it is to follow, heard once |
| `hook` | score 0–4 | How strongly the opening gives a reason to keep watching |
| `difficulty` | score 0–4 | Prior knowledge needed (stored as 1–5) |
| `key_correct` | noul | The marked answer is right according to the script |
| `single_answer` | noul | Exactly one option is right |
| `distractors` | score 0–4 | How believable the wrong options are |
| `tag_N` | noul | A main subject of the script is "{tag}" (writer's tags plus the company's 20 most used) |

Decision (`review`): a 0–100 score weighted towards accuracy (grounded 30, clarity 20, on topic
10, safe 10, hook 10, answer key 10, caption 5, distractors 5), flags for each failed judgement
(probability below 0.5, or score below 2 for clarity, 1.5 for hook and distractors), and a hard cap
below the company's threshold for an unsafe script, an ungrounded script or a wrong answer key, so
averages can never hide them. Tags Jev is at least 60 % sure of are kept (writer's first, then
the company's existing subjects, at most five), which connects new reels to the tags people already
follow. The result is stored on the reel (`reels.ai_review`, `reels.quality_score`) and shown to
reviewers in the library, the editor and the publish screen. It is advisory: people still approve
every reel.

### Job-title relevance (hourly engine job)

For each active job title (normalised), published reels not yet judged are sent in batches of 40,
each reel its own noul question: "Someone working as `role` would use what this reel teaches in
their own job." Results go to `reel_role_fit`. The ranker adds a prior to a person's interest:
`(fit − 0.5) × 2 × 3` interest points, fading linearly to zero over their first 25 signals, so a new
cashier starts from cashier-relevant reels and their own choices take over within weeks. It is
personalisation, so it is off for people who switched personalisation off.

## API

Company admin: `GET/PUT /admin/ai` (routing, plan summary, whose key per provider),
`GET /admin/ai/usage?month=YYYY-MM`, and the existing `/admin/integrations` for the company's own
keys (unknown settings for AI providers are refused). Platform: `GET /platform/me`,
`GET /platform/integrations`, `PUT /platform/integrations/{kind}`, `POST …/test`,
`GET /platform/tenants?month=`, `PUT /platform/tenants/{id}/ai-plan`, `GET /platform/usage.csv`.
Secrets are AES-GCM encrypted at rest and never returned.
