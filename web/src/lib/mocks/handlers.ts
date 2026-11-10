import { HttpResponse, http, type HttpResponseResolver } from "msw";
import type { AssignmentWrite, Export, GenerationRequest, Problem, Reel, ReelPatch, Source, SourceCreate, Tenant, TopicCreate, UserCreate } from "@/lib/api/types";
import { apiBaseUrl } from "@/lib/auth/config";
import { buildQuote } from "@/features/studio/logic/quote";
import { approvalRoleFor, awaitingLabel, canPublish } from "@/features/studio/logic/approvals";
import { previewImport } from "@/features/people/logic/csv";
import { applyLibraryFilters } from "@/features/library/logic/filters";
import { DEV_OTP_CODE, tokenPairFor, userFromToken } from "./auth";
import { db, newId, nowIso, persist, record } from "./db";
import { toListItem } from "./fixtures/content";
import { dashboard, hrisSyncLog, reportOverview, reportsByGroup, reportsByTopic } from "./fixtures/tenant";
import { transcript } from "./fixtures/content";
import { createJob, materializeJob, retryJob } from "./jobs";
import { engineInsights, enginePresets } from "./fixtures/engine";
import { changedFields, labelFor, NUMBER_RANGES, PRESETS, WEIGHT_RANGES, type NumberKey, type WeightKey } from "@/features/engine/logic/engine";
import type { EngineSettings } from "@/lib/api/types";

const BASE = apiBaseUrl();
const url = (path: string) => `${BASE}${path}`;

let requestSeq = 0;
function problem(status: number, title: string, detail?: string, errors?: Problem["errors"]) {
  requestSeq += 1;
  const body: Problem = {
    type: `https://reelearn.example/problems/${title.toLowerCase().replace(/\s+/g, "-")}`,
    title,
    status,
    detail,
    request_id: `req_mock_${String(requestSeq).padStart(5, "0")}`,
    errors,
  };
  return HttpResponse.json(body, { status, headers: { "content-type": "application/problem+json" } });
}

function callerOf(request: Request) {
  return userFromToken(request.headers.get("authorization")?.replace(/^Bearer\s+/i, ""));
}

/** Wrap a resolver so a missing/invalid bearer yields 401 like the real API. */
function authed(resolver: HttpResponseResolver): HttpResponseResolver {
  return (info) => {
    if (!callerOf(info.request)) return problem(401, "Unauthorized", "Missing or expired access token");
    return resolver(info);
  };
}

function reelWithAwaiting(reel: Reel) {
  const label = awaitingLabel(reel);
  const text = label === "both" ? "L&D and SME sign-off" : label === "ld" ? "L&D sign-off" : label === "sme" ? "SME sign-off" : null;
  return toListItem(reel, text);
}

function jobOrNull(id: string) {
  const job = db.jobs.find((j) => j.id === id);
  if (!job) return null;
  return materializeJob(job, db.jobRuns[id]);
}

const latency = () => new Promise((r) => setTimeout(r, 120));

export const handlers = [
  // ---------------------------------------------------------------- system
  http.get(url("/health"), () => HttpResponse.json({ status: "ok", postgres: true, redis: true, version: "mock" })),

  // ---------------------------------------------------------------- auth
  http.post(url("/auth/tenant/lookup"), async ({ request }) => {
    const body = (await request.json()) as { slug?: string };
    if (body.slug !== db.tenant.slug) return problem(404, "Workspace not found", `No workspace with code "${body.slug ?? ""}"`);
    const { id, slug, name, logo_url, languages, region } = db.tenant;
    return HttpResponse.json({ id, slug, name, logo_url, languages, region });
  }),
  http.post(url("/auth/otp/request"), async ({ request }) => {
    await latency();
    const body = (await request.json()) as { tenant_slug: string; email?: string; phone?: string };
    if (body.tenant_slug !== db.tenant.slug) return problem(404, "Workspace not found");
    const user = db.users.find(
      (u) => (body.email && u.email?.toLowerCase() === body.email.toLowerCase()) || (body.phone && u.phone_e164 === body.phone),
    );
    if (!user || user.role === "learner") return problem(404, "Not an admin account", "This email is not an admin, manager or SME in this workspace.");
    const challenge_id = newId();
    db.challenges[challenge_id] = user.id;
    persist();
    return HttpResponse.json({ challenge_id, channel: body.email ? "email" : "whatsapp", expires_in_seconds: 300 });
  }),
  http.post(url("/auth/otp/verify"), async ({ request }) => {
    await latency();
    const body = (await request.json()) as { challenge_id: string; code: string };
    const userId = db.challenges[body.challenge_id];
    const user = db.users.find((u) => u.id === userId);
    if (!user) return problem(400, "Unknown challenge", "Request a new code.");
    if (body.code !== DEV_OTP_CODE) return problem(400, "Wrong code", "The code does not match. 4 attempts left.");
    return HttpResponse.json(tokenPairFor(user));
  }),
  http.post(url("/auth/refresh"), async ({ request }) => {
    const body = (await request.json()) as { refresh_token: string };
    const user = userFromToken(body.refresh_token, "mock-refresh-");
    return user ? HttpResponse.json(tokenPairFor(user)) : problem(401, "Invalid refresh token");
  }),
  http.post(url("/auth/logout"), () => new HttpResponse(null, { status: 204 })),

  // ---------------------------------------------------------------- tenants
  http.post(url("/tenants"), async ({ request }) => {
    await latency();
    const body = (await request.json()) as { slug: string; company_name: string; admin_email: string };
    if (body.slug === db.tenant.slug) return problem(409, "Slug taken", "That workspace code is already in use.", [{ field: "slug", message: "Already in use" }]);
    const challenge_id = newId();
    db.challenges[challenge_id] = db.users[0]?.id ?? "";
    persist();
    return HttpResponse.json(
      { tenant: { id: newId(), slug: body.slug, name: body.company_name, logo_url: null, languages: ["ar", "en"], region: "ksa" }, challenge_id },
      { status: 201 },
    );
  }),
  http.get(url("/me"), authed(({ request }) => HttpResponse.json(tokenPairFor(callerOf(request)!).user))),
  http.get(url("/admin/tenant"), authed(() => HttpResponse.json(db.tenant))),
  http.patch(url("/admin/tenant"), authed(async ({ request }) => {
    const patch = (await request.json()) as Partial<Tenant>;
    db.tenant = { ...db.tenant, ...patch, settings: { ...db.tenant.settings, ...(patch.settings ?? {}) } as Tenant["settings"] };
    record({ actor: null, action: "tenant.updated", entity: "tenant", entity_id: db.tenant.id, diff: patch as never });
    persist();
    return HttpResponse.json(db.tenant);
  })),
  http.get(url("/admin/dashboard"), authed(async () => {
    await latency();
    return HttpResponse.json({ ...dashboard, credits: db.tenant.credits, learners_total: db.users.filter((u) => u.role === "learner").length });
  })),

  // ---------------------------------------------------------------- people
  http.get(url("/admin/users"), authed(({ request }) => {
    const p = new URL(request.url).searchParams;
    const q = p.get("q")?.toLowerCase();
    const items = db.users.filter(
      (u) =>
        (!q || u.full_name.toLowerCase().includes(q) || u.email?.toLowerCase().includes(q) || u.employee_no?.toLowerCase().includes(q)) &&
        (!p.get("role") || u.role === p.get("role")) &&
        (!p.get("status") || u.status === p.get("status")) &&
        (!p.get("language") || u.preferred_language === p.get("language")) &&
        (!p.get("group_id") || u.groups.some((g) => g.id === p.get("group_id"))),
    );
    return HttpResponse.json({ items, next_cursor: null });
  })),
  http.post(url("/admin/users"), authed(async ({ request }) => {
    const body = (await request.json()) as UserCreate;
    if (body.email && db.users.some((u) => u.email?.toLowerCase() === body.email?.toLowerCase())) {
      return problem(409, "Already exists", "A person with this email already exists.", [{ field: "email", message: "Already in use" }]);
    }
    const user = {
      id: newId(),
      role: body.role,
      status: "invited" as const,
      full_name: body.full_name,
      phone_e164: body.phone ?? null,
      email: body.email ?? null,
      employee_no: body.employee_no ?? null,
      preferred_language: body.preferred_language ?? db.tenant.default_language ?? "ar",
      manager_id: body.manager_id ?? null,
      location: body.location ?? null,
      job_title: body.job_title ?? null,
      delivery_channel: body.delivery_channel ?? "app",
      delivery_slot: body.delivery_slot ?? "shift_start",
      groups: db.groups.filter((g) => body.group_ids?.includes(g.id)).map((g) => ({ id: g.id, name: g.name })),
      last_seen_at: null,
      created_at: nowIso(),
    };
    db.users.push(user);
    record({ actor: null, action: "user.invited", entity: "user", entity_id: user.id, diff: { status: [null, "invited"] } as never });
    persist();
    return HttpResponse.json(user, { status: 201 });
  })),
  http.post(url("/admin/users/import"), authed(async ({ request }) => {
    await latency();
    const dryRun = new URL(request.url).searchParams.get("dry_run") === "true";
    const csv = await request.text();
    const preview = previewImport(csv, db.users.map((u) => ({ email: u.email, phone: u.phone_e164, employee_no: u.employee_no })));
    if (!dryRun) {
      preview.rows
        .filter((r) => r.action === "create")
        .forEach((r) => {
          db.users.push({
            id: newId(),
            role: (r.values.role?.toLowerCase() as "learner") ?? "learner",
            status: "invited",
            full_name: r.values.full_name ?? "",
            phone_e164: r.values.phone ?? null,
            email: r.values.email ?? null,
            employee_no: r.values.employee_no ?? null,
            preferred_language: r.values.preferred_language?.toLowerCase() ?? "ar",
            manager_id: null,
            location: r.values.location ?? null,
            job_title: r.values.job_title ?? null,
            delivery_channel: (r.values.delivery_channel?.toLowerCase() as "app") ?? "app",
            delivery_slot: "shift_start",
            groups: [],
            last_seen_at: null,
            created_at: nowIso(),
          });
        });
      record({ actor: null, action: "users.imported", entity: "user", entity_id: null, diff: preview.totals as never });
      persist();
    }
    return HttpResponse.json({
      total_rows: preview.totals.total,
      created: preview.totals.create,
      updated: preview.totals.update,
      skipped: preview.totals.skip,
      errors: preview.rows.filter((r) => r.errors.length).map((r) => ({ row: r.row, message: r.errors.join(", ") })),
      dry_run: dryRun,
    });
  })),
  http.get(url("/admin/users/:user_id"), authed(({ params }) => {
    const user = db.users.find((u) => u.id === params.user_id);
    return user ? HttpResponse.json(user) : problem(404, "Not found");
  })),
  http.patch(url("/admin/users/:user_id"), authed(async ({ params, request }) => {
    const user = db.users.find((u) => u.id === params.user_id);
    if (!user) return problem(404, "Not found");
    const patch = (await request.json()) as Record<string, unknown>;
    const { group_ids, ...rest } = patch as { group_ids?: string[] } & Record<string, unknown>;
    Object.assign(user, rest);
    if (group_ids) user.groups = db.groups.filter((g) => group_ids.includes(g.id)).map((g) => ({ id: g.id, name: g.name }));
    record({ actor: null, action: "user.updated", entity: "user", entity_id: user.id, diff: patch as never });
    persist();
    return HttpResponse.json(user);
  })),
  http.get(url("/admin/groups"), authed(() =>
    HttpResponse.json(db.groups.map((g) => ({ ...g, member_count: db.users.filter((u) => u.groups.some((x) => x.id === g.id)).length }))),
  )),
  http.post(url("/admin/groups"), authed(async ({ request }) => {
    const body = (await request.json()) as { name: string; kind?: string };
    const group = { id: newId(), name: body.name, kind: body.kind ?? "custom", member_count: 0 };
    db.groups.push(group);
    persist();
    return HttpResponse.json(group, { status: 201 });
  })),
  http.put(url("/admin/groups/:group_id/members"), authed(async ({ params, request }) => {
    const group = db.groups.find((g) => g.id === params.group_id);
    if (!group) return problem(404, "Not found");
    const { user_ids } = (await request.json()) as { user_ids: string[] };
    db.users.forEach((u) => {
      const has = u.groups.some((g) => g.id === group.id);
      const should = user_ids.includes(u.id);
      if (should && !has) u.groups.push({ id: group.id, name: group.name });
      if (!should && has) u.groups = u.groups.filter((g) => g.id !== group.id);
    });
    persist();
    return new HttpResponse(null, { status: 204 });
  })),
  // Not in the contract; used by the HRIS sync log panel (R03). Marked clearly as mock-only.
  http.get(url("/admin/integrations/hris/sync-log"), authed(() => HttpResponse.json(hrisSyncLog))),
  http.get(url("/admin/sources/:source_id/transcript"), authed(() => HttpResponse.json(transcript))),

  // ---------------------------------------------------------------- topics
  http.get(url("/admin/topics"), authed(() => HttpResponse.json(db.topics))),
  http.post(url("/admin/topics"), authed(async ({ request }) => {
    const body = (await request.json()) as TopicCreate;
    const topic = {
      id: newId(),
      slug: body.slug ?? body.name.toLowerCase().replace(/[^a-z0-9]+/g, "-"),
      name: body.name,
      name_ar: body.name_ar ?? null,
      description: body.description ?? null,
      compliance_tag: body.compliance_tag ?? null,
      is_mandatory: body.is_mandatory ?? false,
      status: "active",
      reel_counts: { published: 0, in_review: 0, draft: 0 },
      recall_pct: null,
    };
    db.topics.push(topic);
    record({ actor: null, action: "topic.created", entity: "topic", entity_id: topic.id, diff: null });
    persist();
    return HttpResponse.json(topic, { status: 201 });
  })),
  http.patch(url("/admin/topics/:topic_id"), authed(async ({ params, request }) => {
    const topic = db.topics.find((t) => t.id === params.topic_id);
    if (!topic) return problem(404, "Not found");
    Object.assign(topic, await request.json());
    persist();
    return HttpResponse.json(topic);
  })),
  http.get(url("/admin/topics/:topic_id/assignments"), authed(({ params }) =>
    HttpResponse.json(db.assignments.filter((a) => a.topic_id === params.topic_id)),
  )),
  http.put(url("/admin/topics/:topic_id/assignments"), authed(async ({ params, request }) => {
    const topicId = String(params.topic_id);
    const writes = (await request.json()) as AssignmentWrite[];
    const existing = db.assignments.filter((a) => a.topic_id === topicId);
    const next = writes.map((w) => {
      const group = db.groups.find((g) => g.id === w.group_id);
      const prev = existing.find((a) => a.group.id === w.group_id);
      return {
        id: prev?.id ?? newId(),
        topic_id: topicId,
        group: { id: w.group_id, name: group?.name ?? "" },
        reels_per_day: w.reels_per_day ?? 2,
        channel: w.channel ?? "app",
        is_mandatory: w.is_mandatory ?? false,
        starts_on: w.starts_on ?? new Date().toISOString().slice(0, 10),
        ends_on: w.ends_on ?? null,
      };
    });
    db.assignments = [...db.assignments.filter((a) => a.topic_id !== topicId), ...next];
    record({ actor: null, action: "topic.assignment.updated", entity: "topic", entity_id: topicId, diff: { groups: next.map((a) => a.group.name) } as never });
    persist();
    return HttpResponse.json(next);
  })),

  // ---------------------------------------------------------------- studio
  http.post(url("/admin/uploads"), authed(async ({ request }) => {
    const body = (await request.json()) as { filename: string; mime: string; bytes: number };
    if (body.bytes > 209_715_200) return problem(422, "File too large", "Files are limited to 200 MB.", [{ field: "bytes", message: "Over 200 MB" }]);
    return HttpResponse.json(
      { id: newId(), upload_url: `${BASE}/mock-upload/${encodeURIComponent(body.filename)}`, method: "PUT", headers: { "content-type": body.mime }, expires_at: new Date(Date.now() + 600_000).toISOString() },
      { status: 201 },
    );
  })),
  http.put(url("/mock-upload/:name"), () => new HttpResponse(null, { status: 200 })),
  http.get(url("/admin/sources"), authed(() => HttpResponse.json({ items: db.sources, next_cursor: null }))),
  http.post(url("/admin/sources"), authed(async ({ request }) => {
    await latency();
    const body = (await request.json()) as SourceCreate;
    if (body.kind === "url" && body.url?.includes("blocked.example")) {
      return problem(422, "URL blocked", "The site's robots.txt does not allow extraction.", [{ field: "url", message: "Blocked by robots.txt" }]);
    }
    const source: Source = {
      id: newId(),
      kind: body.kind,
      title: body.title,
      url: body.url ?? null,
      language: body.language ?? null,
      extraction_status: "done" as const,
      extraction_error: null,
      summary: {
        key_ideas: ["One idea per reel", "A concrete example", "One line to remember"],
        suggested_reels: [
          { title: `${body.title}: the one rule`, hook: "Start with the thing people get wrong." },
          { title: `${body.title}: a real example`, hook: "Show it on the shop floor." },
          { title: `${body.title}: what to say`, hook: "Give them the exact words." },
        ],
        word_count: 1800,
        duration_ms: body.kind === "podcast_audio" || body.kind === "video" ? 1_200_000 : null,
      } as never,
      created_by: callerOf(request)?.id ?? "",
      created_at: nowIso(),
    };
    db.sources.unshift(source);
    record({ actor: null, action: "source.created", entity: "source", entity_id: source.id, diff: { kind: body.kind } as never });
    persist();
    return HttpResponse.json(source, { status: 201 });
  })),
  http.get(url("/admin/sources/:source_id"), authed(({ params }) => {
    const source = db.sources.find((s) => s.id === params.source_id);
    return source ? HttpResponse.json(source) : problem(404, "Not found");
  })),
  http.get(url("/admin/sources/:source_id/moments"), authed(({ params }) => {
    const source = db.sources.find((s) => s.id === params.source_id);
    if (!source) return problem(404, "Not found");
    return HttpResponse.json(source.id === db.sources.find((s) => s.kind === "podcast_audio")?.id || source.kind === "podcast_audio" ? db.moments : []);
  })),
  http.post(url("/admin/sources/:source_id/moments/:moment_id/cut"), authed(async ({ params, request }) => {
    await latency();
    const body = (await request.json()) as { topic_id: string; start_ms?: number; end_ms?: number; title?: string; speaker_consent: boolean };
    const moment = db.moments.find((m) => m.id === params.moment_id);
    const source = db.sources.find((s) => s.id === params.source_id);
    if (!moment || !source) return problem(404, "Not found");
    if (!body.speaker_consent) return problem(422, "Consent required", "Confirm the speaker's consent before cutting.", [{ field: "speaker_consent", message: "Required" }]);
    const start = body.start_ms ?? moment.start_ms;
    const end = body.end_ms ?? moment.end_ms;
    const reel: Reel = {
      id: newId(),
      topic_id: body.topic_id,
      source_id: source.id,
      generation_job_id: null,
      kind: "podcast_cut",
      status: "draft",
      title: body.title ?? moment.title ?? "Podcast cut",
      language: source.language ?? "ar",
      script: moment.excerpt.replace(/^\.\.\.|\.\.\.$/g, "").trim(),
      source_citation: `${source.title}, ${msToClock(start)}–${msToClock(end)}`,
      presenter: { name: moment.speaker ?? "" } as never,
      video_url: null,
      thumbnail_url: null,
      captions_url: null,
      duration_ms: end - start,
      version: 1,
      compliance_tag: null,
      consent_confirmed: true,
      variants: [],
      question: null,
      approvals: [],
      audience: [],
      recall_pct: null,
      published_at: null,
      created_at: nowIso(),
      updated_at: nowIso(),
    };
    moment.selected = true;
    moment.reel_id = reel.id;
    moment.start_ms = start;
    moment.end_ms = end;
    db.reels.unshift(reel);
    record({ actor: null, action: "reel.cut", entity: "reel", entity_id: reel.id, diff: { moment: moment.id } as never });
    persist();
    return HttpResponse.json(reel, { status: 201 });
  })),
  http.post(url("/admin/generation-jobs/quote"), authed(async ({ request }) => {
    await latency();
    const body = (await request.json()) as GenerationRequest;
    return HttpResponse.json(buildQuote(body, db.tenant.credits ?? 0));
  })),
  http.get(url("/admin/generation-jobs"), authed(() =>
    HttpResponse.json({ items: db.jobs.map((j) => materializeJob(j, db.jobRuns[j.id])), next_cursor: null }),
  )),
  http.post(url("/admin/generation-jobs"), authed(async ({ request }) => {
    await latency();
    const body = (await request.json()) as GenerationRequest;
    const quote = buildQuote(body, db.tenant.credits ?? 0);
    if (quote.cost_credits! > (db.tenant.credits ?? 0)) {
      return problem(402, "Not enough credits", `This job needs ${quote.cost_credits} credits; ${db.tenant.credits} available.`);
    }
    db.tenant.credits = (db.tenant.credits ?? 0) - (quote.cost_credits ?? 0);
    const job = createJob(body, quote.cost_credits ?? 0);
    record({ actor: null, action: "generation_job.created", entity: "generation_job", entity_id: job.id, diff: { cost_credits: job.cost_credits } as never });
    return HttpResponse.json(job, { status: 202 });
  })),
  http.get(url("/admin/generation-jobs/:job_id"), authed(({ params }) => {
    const job = jobOrNull(String(params.job_id));
    return job ? HttpResponse.json(job) : problem(404, "Not found");
  })),
  http.get(url("/admin/generation-jobs/:job_id/events"), authed(({ params }) => {
    const id = String(params.job_id);
    if (!jobOrNull(id)) return problem(404, "Not found");
    const encoder = new TextEncoder();
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        const tick = () => {
          const job = jobOrNull(id);
          if (!job) {
            controller.close();
            return;
          }
          controller.enqueue(encoder.encode(`event: snapshot\ndata: ${JSON.stringify(job)}\n\n`));
          if (job.status === "done" || job.status === "failed" || job.status === "cancelled") {
            controller.close();
            return;
          }
          setTimeout(tick, 400);
        };
        tick();
      },
    });
    return new HttpResponse(stream, { headers: { "content-type": "text/event-stream", "cache-control": "no-cache" } });
  })),
  http.post(url("/admin/generation-jobs/:job_id/retry"), authed(({ params }) => {
    const job = db.jobs.find((j) => j.id === params.job_id);
    if (!job) return problem(404, "Not found");
    return HttpResponse.json(retryJob(job), { status: 202 });
  })),
  http.get(url("/admin/generation-jobs/:job_id/storyboard"), authed(({ params }) => {
    const job = jobOrNull(String(params.job_id));
    if (!job) return problem(404, "Not found");
    return HttpResponse.json(db.reels.filter((r) => job.reel_ids?.includes(r.id)));
  })),

  // ---------------------------------------------------------------- reels
  http.get(url("/admin/reels"), authed(async ({ request }) => {
    await latency();
    const p = new URL(request.url).searchParams;
    const items = applyLibraryFilters(db.reels.map(reelWithAwaiting), {
      status: p.get("status") ?? undefined,
      topic_id: p.get("topic_id") ?? undefined,
      language: p.get("language") ?? undefined,
      kind: p.get("kind") ?? undefined,
      q: p.get("q") ?? undefined,
    });
    return HttpResponse.json({ items, next_cursor: null });
  })),
  http.get(url("/admin/reels/:reel_id"), authed(({ params }) => {
    const reel = db.reels.find((r) => r.id === params.reel_id);
    return reel ? HttpResponse.json(reel) : problem(404, "Not found");
  })),
  http.patch(url("/admin/reels/:reel_id"), authed(async ({ params, request }) => {
    await latency();
    const reel = db.reels.find((r) => r.id === params.reel_id);
    if (!reel) return problem(404, "Not found");
    const patch = (await request.json()) as ReelPatch;
    const wasPublished = reel.status === "published";
    if (patch.title !== undefined) reel.title = patch.title;
    if (patch.script !== undefined) reel.script = patch.script;
    if (patch.source_citation !== undefined) reel.source_citation = patch.source_citation;
    if (patch.compliance_tag !== undefined) reel.compliance_tag = patch.compliance_tag;
    if (patch.presenter !== undefined) reel.presenter = patch.presenter as never;
    if (patch.question) {
      reel.question = {
        id: reel.question?.id ?? newId(),
        language: reel.question?.language ?? reel.language,
        prompt: patch.question.prompt ?? reel.question?.prompt ?? "",
        options: (patch.question.options ?? reel.question?.options ?? []).map((o) => ({ key: o.key ?? "", text: o.text ?? "" })),
        correct_key: patch.question.correct_key ?? reel.question?.correct_key,
        explanation: patch.question.explanation ?? reel.question?.explanation ?? null,
      };
    }
    if (wasPublished) {
      reel.version += 1;
      reel.status = "in_review";
    }
    reel.updated_at = nowIso();
    record({ actor: null, action: "reel.updated", entity: "reel", entity_id: reel.id, diff: { version: reel.version } as never });
    persist();
    return HttpResponse.json(reel);
  })),
  http.post(url("/admin/reels/:reel_id/regenerate"), authed(async ({ params, request }) => {
    const reel = db.reels.find((r) => r.id === params.reel_id);
    if (!reel) return problem(404, "Not found");
    const body = (await request.json().catch(() => ({}))) as { what?: string[] };
    const cost = (body.what ?? ["visual"]).reduce((n, w) => n + (w === "visual" ? 12 : w === "voice" ? 4 : 2), 0);
    if (cost > (db.tenant.credits ?? 0)) return problem(402, "Not enough credits");
    db.tenant.credits = (db.tenant.credits ?? 0) - cost;
    const job = createJob({ source_id: reel.source_id ?? "", topic_id: reel.topic_id, languages: [reel.language], reel_count: 1, style: reel.kind === "motion" ? "motion" : reel.kind === "broll" ? "broll" : "ai_avatar" }, cost);
    job.reel_ids = [reel.id];
    persist();
    return HttpResponse.json(job, { status: 202 });
  })),
  http.post(url("/admin/reels/:reel_id/variants"), authed(async ({ params, request }) => {
    const reel = db.reels.find((r) => r.id === params.reel_id);
    if (!reel) return problem(404, "Not found");
    const { languages } = (await request.json()) as { languages: string[] };
    reel.variants = [...(reel.variants ?? []), ...languages.map((language) => ({ language, dub_status: "queued", video_url: null, captions_url: null }))];
    persist();
    const job = createJob({ source_id: reel.source_id ?? "", topic_id: reel.topic_id, languages: [reel.language, ...languages], reel_count: 1, style: "ai_avatar" }, languages.length * 5);
    job.reel_ids = [reel.id];
    return HttpResponse.json(job, { status: 202 });
  })),
  http.post(url("/admin/reels/:reel_id/submit-review"), authed(({ params }) => {
    const reel = db.reels.find((r) => r.id === params.reel_id);
    if (!reel) return problem(404, "Not found");
    reel.status = "in_review";
    reel.updated_at = nowIso();
    record({ actor: null, action: "reel.submitted", entity: "reel", entity_id: reel.id, diff: null });
    persist();
    return HttpResponse.json(reel);
  })),
  http.post(url("/admin/reels/:reel_id/approvals"), authed(async ({ params, request }) => {
    await latency();
    const reel = db.reels.find((r) => r.id === params.reel_id);
    if (!reel) return problem(404, "Not found");
    const caller = callerOf(request)!;
    const role = approvalRoleFor(caller.role);
    if (!role) return problem(403, "Forbidden", "Only L&D (admin/manager) and SME users can record a decision.");
    const body = (await request.json()) as { decision: "approved" | "changes_requested" | "rejected"; comment?: string };
    reel.approvals = [
      ...(reel.approvals ?? []).filter((a) => !(a.role === role && a.reel_version === reel.version)),
      { role, decision: body.decision, reviewer_name: caller.full_name, comment: body.comment ?? null, decided_at: nowIso(), reel_version: reel.version },
    ];
    if (body.decision === "rejected") reel.status = "rejected";
    else if (body.decision === "changes_requested") reel.status = "draft";
    else if (reel.status === "draft") reel.status = "in_review";
    if (canPublish(reel, 1) && reel.status === "in_review") reel.status = "approved";
    reel.updated_at = nowIso();
    record({ actor: { id: caller.id, full_name: caller.full_name }, action: `reel.${body.decision}`, entity: "reel", entity_id: reel.id, diff: { role } as never });
    persist();
    return HttpResponse.json(reel);
  })),
  http.post(url("/admin/reels/:reel_id/publish"), authed(async ({ params, request }) => {
    await latency();
    const reel = db.reels.find((r) => r.id === params.reel_id);
    if (!reel) return problem(404, "Not found");
    const body = (await request.json()) as { group_ids: string[]; starts_on?: string; compliance_tag?: string };
    if (!canPublish(reel, body.group_ids.length)) {
      return problem(409, "Approvals missing", "Both L&D and SME must approve this version before it is published.");
    }
    reel.status = "published";
    reel.audience = db.groups.filter((g) => body.group_ids.includes(g.id)).map((g) => ({ id: g.id, name: g.name }));
    reel.compliance_tag = body.compliance_tag ?? reel.compliance_tag ?? null;
    reel.published_at = body.starts_on ? `${body.starts_on}T07:00:00Z` : nowIso();
    reel.updated_at = nowIso();
    const topic = db.topics.find((t) => t.id === reel.topic_id);
    if (topic) topic.reel_counts.published = (topic.reel_counts.published ?? 0) + 1;
    record({ actor: null, action: "reel.published", entity: "reel", entity_id: reel.id, diff: { audience: reel.audience.map((a) => a.name) } as never });
    persist();
    return HttpResponse.json(reel);
  })),
  http.post(url("/admin/reels/:reel_id/archive"), authed(({ params }) => {
    const reel = db.reels.find((r) => r.id === params.reel_id);
    if (!reel) return problem(404, "Not found");
    reel.status = "archived";
    reel.updated_at = nowIso();
    persist();
    return HttpResponse.json(reel);
  })),

  // ---------------------------------------------------------------- reports
  http.get(url("/admin/reports/overview"), authed(() => HttpResponse.json(reportOverview))),
  http.get(url("/admin/reports/by-topic"), authed(() => HttpResponse.json(reportsByTopic))),
  http.get(url("/admin/reports/by-group"), authed(() => HttpResponse.json(reportsByGroup))),
  http.get(url("/admin/exports"), authed(() => HttpResponse.json(db.exports))),
  http.post(url("/admin/exports"), authed(async ({ request }) => {
    await latency();
    const body = (await request.json()) as { kind: string; period_start: string; period_end: string };
    const exp: Export = { id: newId(), kind: body.kind, status: "queued", period_start: body.period_start, period_end: body.period_end, created_at: nowIso(), finished_at: null };
    db.exports.unshift(exp);
    persist();
    setTimeout(() => {
      exp.status = "ready";
      exp.finished_at = nowIso();
      persist();
    }, 2500);
    return HttpResponse.json(exp, { status: 202 });
  })),
  http.get(url("/admin/exports/:export_id/download"), authed(({ params }) =>
    HttpResponse.json({ url: `${BASE}/mock-download/${params.export_id}.zip`, expires_at: new Date(Date.now() + 3_600_000).toISOString() }),
  )),

  // ---------------------------------------------------------------- settings
  http.get(url("/admin/integrations"), authed(() => HttpResponse.json(db.integrations))),
  http.get(url("/admin/engine"), authed(() => HttpResponse.json({ settings: db.engine ?? enginePresets.balanced, presets: enginePresets }))),
  http.put(url("/admin/engine"), authed(async ({ request }) => {
    const body = (await request.json()) as Record<string, unknown>;
    const current = db.engine ?? enginePresets.balanced!;
    let next: EngineSettings;
    const keys = Object.keys(body);
    if (keys.length === 1 && typeof body.intensity === "string" && (PRESETS as string[]).includes(body.intensity)) {
      next = enginePresets[body.intensity]!;
    } else {
      const { intensity: _ignored, weights, ...rest } = body as Partial<EngineSettings>;
      void _ignored;
      next = { ...current, ...rest, weights: { ...current.weights, ...(weights ?? {}) } } as EngineSettings;
      const errors: { field: string; message: string }[] = [];
      for (const [k, r] of Object.entries(WEIGHT_RANGES)) {
        const v = next.weights[k as WeightKey] ?? 0;
        if (v < r.min || v > r.max) errors.push({ field: `weights.${k}`, message: `must be between ${r.min} and ${r.max}` });
      }
      for (const [k, r] of Object.entries(NUMBER_RANGES)) {
        const v = next[k as NumberKey];
        if (v < r.min || v > r.max) errors.push({ field: k, message: `must be between ${r.min} and ${r.max}` });
      }
      if (errors.length > 0) return problem(422, "Validation failed", "One or more fields are invalid", errors);
      next = { ...next, intensity: labelFor(next, enginePresets) };
    }
    record({ actor: null, action: "engine.update", entity: "tenant", entity_id: null, diff: { changed: changedFields(current, next) } as never });
    db.engine = next;
    persist();
    return HttpResponse.json({ settings: next, presets: enginePresets });
  })),
  http.get(url("/admin/engine/insights"), authed(({ request }) => {
    const range = new URL(request.url).searchParams.get("range") ?? "30d";
    return HttpResponse.json({ ...engineInsights, range, intensity: (db.engine ?? enginePresets.balanced!).intensity });
  })),
  http.put(url("/admin/integrations/:kind"), authed(async ({ params, request }) => {
    const body = (await request.json()) as Record<string, unknown>;
    const publicConfig = Object.fromEntries(Object.entries(body).filter(([k]) => !/secret|token|key|password/i.test(k)));
    const existing = db.integrations.find((i) => i.kind === params.kind);
    const next = { kind: params.kind as never, status: "connected" as const, last_sync_at: null, last_error: null, public_config: { ...(existing?.public_config ?? {}), ...publicConfig } };
    if (existing) Object.assign(existing, next);
    else db.integrations.push(next);
    record({ actor: null, action: "integration.updated", entity: "integration", entity_id: null, diff: { kind: params.kind } as never });
    persist();
    return HttpResponse.json(next);
  })),
  http.post(url("/admin/integrations/:kind/test"), authed(async ({ params }) => {
    await latency();
    const integration = db.integrations.find((i) => i.kind === params.kind);
    const ok = integration?.status === "connected";
    return HttpResponse.json({ ok, message: ok ? "Connection OK (142 ms)" : integration?.last_error ?? "Not connected" });
  })),
  http.get(url("/admin/audit"), authed(({ request }) => {
    const p = new URL(request.url).searchParams;
    const items = db.audit.filter((a) => (!p.get("entity") || a.entity === p.get("entity")) && (!p.get("entity_id") || a.entity_id === p.get("entity_id")));
    return HttpResponse.json({ items, next_cursor: null });
  })),
];

function msToClock(ms: number): string {
  const total = Math.floor(ms / 1000);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}
