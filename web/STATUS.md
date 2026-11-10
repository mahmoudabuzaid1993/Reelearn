# Web console status

Last verified 2026-11-04 on this machine (Node 24, pnpm 10, Playwright chromium).

| Check | Result |
|---|---|
| `pnpm lint` (eslint, zero warnings) | clean |
| `pnpm typecheck` | clean |
| `pnpm test` (vitest) | 12 files, 43 tests passing |
| `pnpm e2e` (Playwright, mock API) | 13 scenarios passing |

## Screens

| Route | Screen (canvas id) | State | Covered by |
|---|---|---|---|
| `/signin` | OTP sign-in by e-mail, dev code 1234 | done | e2e: anonymous redirect, wrong code shows problem + request id |
| `/signup` | A01 Create workspace | done | e2e: validation, hand-off to OTP |
| `/onboarding/topics` | A02 Topics & people | done | e2e: renders |
| `/onboarding/delivery` | A03 Delivery & schedule | done | e2e: renders |
| `/dashboard` | A04 Dashboard | done | e2e: fixture numbers |
| `/studio/new` | S01 Pick a source | done | e2e: full studio flow |
| `/studio/jobs/new` | S02 Generation setup with credit quote | done | unit: quote arithmetic; e2e |
| `/studio/jobs/[id]` | S03 Generating (SSE, polling fallback) | done | unit: job-status hook; e2e: retry failed step |
| `/studio/jobs/[id]/storyboard` | S04 Storyboard | done | e2e |
| `/studio/sources/[id]/moments` | S05 Podcast cut | done | e2e: consent required |
| `/studio/reels/[id]` | S06 Reel editor with phone preview | done | e2e |
| `/studio/reels/[id]/publish` | S07 Approvals + publish | done | unit: approval gating; e2e: blocked until both approvals |
| `/studio` | Studio home (sources, jobs) | done | — |
| `/topics` | R01 Topics & channels matrix | done | unit: matrix reducer; e2e |
| `/library` | R02 Reel library | done | e2e: filters, search, awaiting states |
| `/people` | R03 People, groups, CSV import dry-run | done | unit: CSV parsing; e2e |
| `/reports` | R04 Reports & HRDF export | done | e2e |
| `/settings` | R05 Integrations, residency, credits | done | e2e |
| `/audit` | Audit log | done | e2e |
| Arabic RTL | `ar` locale mirrors the console | done | unit: direction + tabs RTL; e2e |

## API paths used

All data access goes through the generated client (`src/lib/api`, from `../openapi/openapi.yaml`): auth (`/auth/*`, `/tenants`), `/admin/tenant`, `/admin/dashboard`, `/admin/users*`, `/admin/groups*`, `/admin/topics*`, `/admin/uploads`, `/admin/sources*`, `/admin/generation-jobs*`, `/admin/reels*`, `/admin/reports/*`, `/admin/exports*`, `/admin/integrations*`, `/admin/audit`.

## Not yet verified

- The console has been exercised against the MSW mock API only (`NEXT_PUBLIC_API_MOCK=true`); the live-API run is part of the integration step in the repository status.

## Learning engine (2026-11-09)

- Learning engine page (`/engine`): intensity presets, value-model weights, practice, discovery, learning and reminder settings, and an insights tab; mock handlers; vitest for the settings logic and Playwright for preset, tuning and insights.
