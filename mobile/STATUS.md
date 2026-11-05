# Mobile status

Last verified 2026-11-04 on this machine with Flutter 3.47.9 (stable) / Dart 3.11.

| Check | Result |
|---|---|
| `tool/scaffold.sh` (flutter create ios+android, deep link `reelearn://t/<slug>`, camera/mic permissions) | done |
| `flutter pub get` | resolves |
| `dart format --set-exit-if-changed lib test` | clean |
| `flutter analyze` | no issues |
| `flutter test` | 52 tests passing |

## Screens

| Route | Screen | State | Tests |
|---|---|---|---|
| `/signin` | Sign-in: company code / deep link, +966 phone, "Send my code on WhatsApp", OTP, lockout | done | `signin_flow_test` (OTP 1234 succeeds, wrong code with attempts left, 5 wrong codes lock, unknown company/phone messages, deep-link slug skips the company step) |
| `/welcome` | First open: 3 promises, slot picker, "Start my first minute" (PATCH /me) | done | session `needsWelcome` covered in the sign-in test |
| `/today` | Plan summary, streak chip, yesterday line, empty / done states | done | `today_screens_test` |
| `/today/play` | **The swipe stack** (reel, question, question_only, done cards) | done | `swipe_stack_test` (renders, forward gating toast and stays, answered allows forward, backward always, done card last with nothing after, reduced motion, segment states, 90 % watch event once, close, RTL), `stack_model_test`, `question_card_test` (pick once, mint reveal, no red anywhere in the tree, offline note) |
| `/today/done` | Done summary (mint check, remembered X of N, streak, freeze note, Back to work, Saved) | done | `today_screens_test` |
| `/saved` | Grid, search, topic chips, saved-only, save toggles, anonymous "Ask for a reel" | done | — |
| `/progress` | "You remember N %" headline, weekly bars, by topic, milestones | done | — |
| `/settings` | Language ar/en/ur/bn, slot, channel, captions, reminders, streak freeze, extra reels, pause 2 weeks, "What Al Wadi Retail can see", data export, sign out | done | `settings_test` (patch captions/language/channel, pause/resume, export) |
| `/team` | Readiness per member per topic (ready / not yet / not started), team recall %, nudge once per person per day | done | `team_test` (rendering, nudge sends once then disabled, API skips repeats) |
| `/team/record` | 60 s recording with 3-2-1 countdown, 3-line teleprompter, consent checkbox, pre-signed PUT upload, submit | done (camera path needs a device) | `record_state_test` (state machine) |

## Core

| Piece | State | Tests |
|---|---|---|
| Models for every schema in the contract (hand-written `fromJson`/`toJson`) | done | exercised by every test through `FakeApi` |
| `ReelearnApi` interface, `HttpApi` (Dio), `FakeApi` (deterministic demo data: Al Wadi, Sara, Khalid, the three plan items, team of four) | done | — |
| Dio interceptors: request id, idempotency safety net, auth + single-flight refresh on 401 | done | `auth_interceptor_test` |
| Offline queue (shared_preferences), replay once on reconnect, stable idempotency keys, 4xx drop | done | `offline_queue_test` |
| Telemetry: watch on arrival + 90 %, answer on pick, offline answer queued with "answer saved" note | done | same file |
| Session (secure-storage tokens, cached profile, welcome once per device), go_router redirects, role-gated manager routes | done | — |
| Localisation en / ar / ur (bn falls back to en), RTL mirroring | done | `l10n_test` |

## Run

```bash
flutter run --dart-define=API=fake          # no backend needed
flutter run --dart-define=API_BASE_URL=http://10.0.2.2:8080/v1   # Android emulator against the local API
```

## Not yet verified

- Camera recording and video playback on a physical device or simulator (widget tests use a stub media builder).
- The app against the live API end to end (the FakeApi mirrors the seed; the HTTP client is exercised by the interceptor tests only).
