# UX flow analysis

Source of truth for screens: the Reelearn UI Flows canvas (30 screens, 6 flows). This document turns those screens into routes, states, data needs and edge cases for the web and mobile builds.

## Principles that every screen must honour

1. **Finite by design.** Every learner session has an end. The stack has a last card; nothing autoplays past it; "Done for today" is a real state, not a suggestion.
2. **A minute, not a module.** Default is 2 new reels + up to 1 review, ~75 s. The product never asks for more; "Saved" browsing is optional and never counted.
3. **No red.** A wrong answer shows the right answer in mint and a "comes back in N days" line. No buzz, no cross, no score on the card.
4. **Privacy as a data rule.** Managers see readiness per topic and team averages; HR sees hours on mandatory topics. Nobody sees which answer a learner chose, what they saved, when they watch, or their location. The API enforces this, the UI states it (E09, A02).
5. **Shift-aware.** Deliveries only inside the learner's shift window, never in prayer times or after hours; Ramadan calendars shift windows automatically (A03).
6. **Language first.** Arabic and English are peers (RTL/LTR switch per user, not per tenant). Urdu, Bengali, Hindi, Tagalog are learner languages; the admin console is Arabic/English only.
7. **Voice.** Never "scroll", "feed", "binge", "addictive" in product copy. The stack is "today's reels"; the gesture is "swipe up".

## Flow 6 · Employee (Flutter)

| Step | Screen | Route | Needs | Edge cases handled |
|---|---|---|---|---|
| Invite | E01 WhatsApp | (outside app) | tenant slug in deep link `reelearn://t/alwadi?inv=…` | link opened on a phone without the app → store page then deep link resumes |
| Sign in | E02a | `/signin` | `POST /auth/tenant/lookup`, `/auth/otp/request`, `/auth/otp/verify` | wrong tenant (company code entry), number not in HR list (clear message, "ask your manager"), OTP expired, 5 wrong attempts (lock 15 min), no WhatsApp (fallback SMS if tenant enables) |
| First open | E02b | `/welcome` | `PATCH /me` (delivery_slot, language) | shown once per device; skipped for returning users |
| Today | E03 | `/today` | `GET /me/today` | plan empty (day off, nothing assigned) → calm empty state; plan completed → done summary; offline → last cached plan with reels pre-downloaded |
| Reels stack | E04 | `/today/play` | plan items; `POST /me/watch`, `POST /me/answers` | answer gating (cannot pass a question unanswered); swipe back allowed; app backgrounded mid-reel resumes at the same card; network loss queues watch/answer events with idempotency keys and replays |
| Recall | (card in E04) | — | question in learner language | wrong → mint reveal, no retry on the same card; question-only review items have no video |
| Done | (last card) | `/today/done` | `user_stats` | streak freeze explained; Friday never counts against streak |
| Saved & topics | E07 | `/saved` | `GET /me/library`, saved toggles, `POST /me/reel-requests` | search across languages; request text is anonymous to the manager |
| Progress | E08 | `/progress` | `GET /me/progress` | shows "remembered", never "watched minutes" as the headline |
| Settings | E09 | `/settings` | `PATCH /me`, `GET /me/privacy`, `POST /me/export` | pause for holiday (2 weeks) suspends deliveries and streak |
| WhatsApp-only | E10 | (bot) | same plan via `/webhooks/whatsapp` | identical items, same gating: the bot does not send the next reel until the question is answered |

### The swipe stack (E04) in detail

- One vertical pager, one card per plan item, plus the Done card. Cards: `reel`, `question`, `question_only` (review), `done`.
- Gesture: vertical drag with follow, release threshold 1/3 of height or a fast flick; rubber-band at both ends and when gated.
- Gating: a `question` card blocks forward navigation until answered; a toast says "Answer first, then swipe up". Backward is always allowed.
- Reel card: video plays on arrival (muted by default on WhatsApp web view, sound on in the app), captions on by default, actions: listen-only, save, captions, "why am I seeing this" (shows topic + assigning group + source citation).
- Progress segments at the top: one per unit (reel + its question = one unit; a review is one unit): upcoming / active / remembered.
- On the last card the stack does not load more. Ever.
- Telemetry: `watch` event on arrival and on 90 % completion; `answer` event on pick. Both carry the plan item id for idempotency.

## Flow 5 · Manager (Flutter, same app, role-gated)

| Screen | Route | Needs | Notes |
|---|---|---|---|
| M01 Team readiness | `/team` | `GET /manager/team` | readiness per person per topic (ready / not yet / not started), team recall average, "nudge" sends a WhatsApp reminder, max 1 per person per day |
| M02 Record a reel | `/team/record` | `POST /sme/recordings` (pre-signed upload), `/submit` | 60 s cap, teleprompter from a 3-line prompt, consent checkbox for face/voice; creates a draft reel into the studio review queue |

## Flow 1 · Admin setup (web)

| Screen | Route | Needs |
|---|---|---|
| A01 Create workspace | `/signup` | `POST /tenants` (company, admin, language, region) then OTP |
| A02 Topics & people | `/onboarding/topics` | topic templates by sector; import people from Jisr/Bayzat/CSV (preview, mapping, errors per row) |
| A03 Delivery & schedule | `/onboarding/delivery` | channel per group, shift windows, prayer/Ramadan toggles, start date |
| A04 Dashboard | `/dashboard` | `GET /admin/dashboard`: weekly actives, recall, the client's KPI, HRDF hours, attention list |

## Flows 2 & 3 · AI Reel Studio (web)

| Screen | Route | Needs | Edge cases |
|---|---|---|---|
| S01 Pick a source | `/studio/new` | `POST /admin/sources` (upload pre-sign, url, youtube, topic prompt, podcast) | unsupported file, >200 MB, URL blocked by robots, YouTube without captions (ASR fallback), scanned PDF (OCR) |
| S02 Generation setup | `/studio/jobs/new?source=` | languages, reel count, style (avatar / motion / b-roll), voice, reviewer; `POST /admin/generation-jobs` returns the credit cost before confirming | insufficient credits → explain and link to plan |
| S03 Generating | `/studio/jobs/:id` | SSE `/admin/generation-jobs/:id/events` | step failed → retry that step only; provider outage → job paused, not failed |
| S04 Storyboard | `/studio/jobs/:id/storyboard` | reels of the job with script, visual, question; edit, regenerate one, approve all | regenerate costs credits again (shown) |
| S05 Podcast cut | `/studio/sources/:id/moments` | transcript, AI moments, trim, consent | speaker consent required before publish |
| S06 Reel editor | `/studio/reels/:id` | `PATCH /admin/reels/:id` script, voice, visual, captions, question, phone preview | editing a published reel creates a new version; learners who already saw v1 keep their memory state |
| S07 Approve & publish | `/studio/reels/:id/publish` | approvals (L&D + SME), audience groups, start date, compliance tag | publish blocked until both approvals; "awaiting SME" state with a nudge |

## Flow 4 · Run (web)

| Screen | Route | Needs |
|---|---|---|
| R01 Topics & channels | `/topics` | assignments matrix, cadence, channel |
| R02 Reel library | `/library` | filters (status, topic, language, kind), search, recall score, "regenerate for 2026" |
| R03 People & groups | `/people` | list, groups, languages, channel, status, HRIS sync log |
| R04 Reports & HRDF | `/reports` | by topic, by group, evidence pack export job |
| R05 Settings & integrations | `/settings` | SSO, HRIS, WhatsApp, AI providers, residency, credits |

## Cross-cutting UX rules for the builders

- Loading: skeletons, never spinners over content; optimistic save on learner actions.
- Errors: inline, human, in the user's language; the request id is shown in a small "copy" affordance for support.
- Empty states carry the next action.
- RTL: every screen mirrored when the UI language is Arabic or Urdu; icons that imply direction (back, next) flip.
- Accessibility: 44 px targets, 4.5:1 contrast, captions default on, reduced-motion respected (no card animation, instant snap).
- Offline (mobile): today's plan and its media are cached on open; events replay with idempotency keys.
