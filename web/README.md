# Reelearn web

The corporate admin console and the AI Reel Studio: Next.js 15 (App Router, TypeScript strict), Tailwind CSS v4, TanStack Query v5, react-hook-form + zod, next-intl (ar / en, full RTL), a typed API client generated from `../openapi/openapi.yaml`, MSW v2 fixtures for API-less development, vitest + Testing Library, Playwright.

## Run

```bash
pnpm install
cp .env.example .env.local          # NEXT_PUBLIC_API_MOCK=true by default
pnpm dev                            # http://localhost:3000
```

Demo sign-in (mock mode): workspace `alwadi`, email `admin@alwadi.example` (admin), `khalid@alwadi.example` (manager) or `hana@alwadi.example` (SME); one-time code `1234`.

## Environment variables

| Variable | Default | Meaning |
|---|---|---|
| `NEXT_PUBLIC_API_URL` | `http://localhost:8080/v1` | Base URL of the Reelearn API (`servers[0]` in the contract). |
| `NEXT_PUBLIC_API_MOCK` | `false` (`true` in `.env.example`) | Serve every API call from MSW handlers backed by the Al Wadi Retail fixtures. No API needed. |
| `REELEARN_REFRESH_COOKIE` | `rl_refresh` | Name of the httpOnly cookie that holds the refresh token. |

## Mock mode

`NEXT_PUBLIC_API_MOCK=true` turns on `src/lib/mocks`:

- `fixtures/` — the demo tenant (Al Wadi Retail, Noura Al-Harbi, Khalid Mansour, Hana Yusuf, four learners, three groups, three topics, six reels, two jobs, a podcast with moments and a transcript, 1,200 credits). Ids are deterministic (`fixtures/ids.ts`) so tests and the API seed can line up.
- `db.ts` — a mutable copy of the fixtures persisted in `localStorage` so multi-page flows survive reloads. Clear `reelearn.mockdb.v1` (or sign out and back in) to reset.
- `handlers.ts` — one MSW handler per contract path, including RFC 9457 problems with `request_id`, 402 on insufficient credits, 409 on publish without both approvals, and an SSE stream for `/generation-jobs/{id}/events`.
- `jobs.ts` — generation jobs advance on wall-clock time (about 0.9 s per step), so progress is the same over SSE, polling and after a reload.
- The browser registers `public/mockServiceWorker.js`; where service workers are refused (some embedded browsers) it falls back to resolving the same handlers inside `window.fetch` (`fetch-fallback.ts`).
- Two convenience endpoints are **not** in the contract and only exist in the mock: `GET /admin/integrations/hris/sync-log` (R03 sync log) and `GET /admin/sources/{id}/transcript` (S05 transcript). Both are isolated in `src/lib/api/hooks/people.ts` and `studio.ts`, so swapping them is one line each when the API adds them.

The mock route handlers under `src/app/api/auth/session` also short-circuit in mock mode so the refresh cookie works without an API.

## Auth model

Admin access tokens never touch `localStorage`: they live in memory (`src/lib/auth/token-store.ts`). The refresh token is an httpOnly cookie set and rotated by Next route handlers (`/api/auth/session`, `/api/auth/session/refresh`) which are the only code that calls `/auth/refresh`. `src/middleware.ts` redirects anyone without the cookie to `/signin` and keeps signed-in users away from it. The openapi-fetch middleware refreshes proactively when the token is within 15 s of expiry.

Because tokens live in memory, server components render the shell, metadata and layout while data is fetched on the client through TanStack Query hooks in `src/lib/api/hooks`.

## Scripts

| Command | What |
|---|---|
| `pnpm dev` / `pnpm build` / `pnpm start` | Next.js. `build` emits a standalone server used by the Dockerfile. |
| `pnpm lint` | ESLint (next/core-web-vitals + typescript-eslint), zero warnings allowed. |
| `pnpm typecheck` | `tsc --noEmit`. |
| `pnpm test` | vitest with v8 coverage over the stateful logic (quote arithmetic, approval gating, CSV preview, matrix reducer, SSE/polling hook, RTL direction, token store, zod schemas). |
| `pnpm e2e` | Playwright (chromium) against `next dev -p 3100` with `NEXT_PUBLIC_API_MOCK=true`. Run `pnpm exec playwright install chromium` once. |
| `pnpm api:types` | Regenerate `src/lib/api/schema.d.ts` from `../openapi/openapi.yaml`. |

## Layout

```
src/app                 routes (route groups: (auth), (onboarding), (console)) + /api/auth/session handlers
src/features/<area>     auth, onboarding, dashboard, studio, library, topics, people, reports, settings, audit
  <area>/logic          pure business rules (no React), unit-tested
  <area>/*.schema.ts    zod schemas
src/components/ui       Button, Input/Select/Textarea/Checkbox/Switch, Field, Dialog, Table, Badge, Tabs, Toast, Skeleton, EmptyState, Card, Stat, PageHeader, ErrorNotice, icons
src/components/layout   console shell, sidebar, topbar, locale switch
src/lib/api             generated schema, typed client, errors, query hooks
src/lib/auth            token store, refresh, provider, server helpers, config
src/lib/mocks           MSW fixtures, db, handlers, worker, fetch fallback
src/i18n                locales, direction helpers, request config, locale action
messages/{en,ar}.json   message catalogs (identical key sets)
e2e/                    Playwright specs
```

## Docker

```bash
docker build -t reelearn-web --build-arg NEXT_PUBLIC_API_URL=https://api.example/v1 .
docker run -p 3000:3000 reelearn-web
```
