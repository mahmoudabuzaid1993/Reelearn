import type { GenerationJob, GenerationRequest, Reel } from "@/lib/api/types";
import { db, newId, nowIso, persist, type JobRun } from "./db";

export const STEP_MS = 900;

function stepsFor(request: GenerationRequest): string[] {
  const base = ["extract", "script", "question", "render", "captions"];
  return request.languages.length > 1 ? [...base, "dub"] : base;
}

function providerFor(step: string, style: GenerationRequest["style"]): string {
  switch (step) {
    case "extract":
    case "script":
    case "question":
      return "claude";
    case "render":
      return style === "motion" ? "veo" : "higgsfield";
    case "captions":
      return "asr";
    case "dub":
      return "voice";
    default:
      return "worker";
  }
}

export function createJob(request: GenerationRequest, cost: number): GenerationJob {
  const steps = stepsFor(request);
  const job: GenerationJob = {
    id: newId(),
    source_id: request.source_id,
    topic_id: request.topic_id,
    status: "queued",
    progress_pct: 0,
    cost_credits: cost,
    params: request as never,
    error: null,
    steps: steps.map((step, i) => ({
      position: i + 1,
      step,
      status: "pending",
      provider: providerFor(step, request.style),
      attempt: 0,
      started_at: null,
      finished_at: null,
      log: null,
    })),
    reel_ids: [],
    created_at: nowIso(),
    finished_at: null,
  };
  db.jobs.unshift(job);
  db.jobRuns[job.id] = { startedAt: Date.now(), stepMs: STEP_MS, resumeFrom: 0 };
  persist();
  return job;
}

function materializeReels(job: GenerationJob): void {
  if ((job.reel_ids?.length ?? 0) > 0) return;
  const params = job.params as unknown as GenerationRequest;
  const source = db.sources.find((s) => s.id === job.source_id);
  const suggested = (source?.summary as { suggested_reels?: { title: string; hook: string }[] } | null)?.suggested_reels ?? [];
  const reels: Reel[] = Array.from({ length: params.reel_count }, (_, i) => {
    const s = suggested[i % Math.max(1, suggested.length)];
    const title = s?.title ?? `${source?.title ?? "Reel"} · part ${i + 1}`;
    const hook = s?.hook ?? "A short, practical reel generated from your source.";
    const id = newId();
    return {
      id,
      topic_id: job.topic_id ?? "",
      source_id: job.source_id,
      generation_job_id: job.id,
      kind: params.style,
      status: "draft",
      title,
      language: params.languages[0] ?? "en",
      script: `${hook} ${s ? "" : ""}Keep it to one idea, one example, one line to remember.`,
      source_citation: source ? `${source.title}` : null,
      presenter: (params.presenter ?? {}) as never,
      video_url: null,
      thumbnail_url: null,
      captions_url: null,
      duration_ms: 30_000 + i * 4_000,
      version: 1,
      compliance_tag: null,
      consent_confirmed: true,
      variants: params.languages.slice(1).map((language) => ({ language, dub_status: "ready", video_url: null, captions_url: null })),
      question: {
        id: newId(),
        language: params.languages[0] ?? "en",
        prompt: `What is the one thing to remember from "${title}"?`,
        options: [
          { key: "A", text: "The first idea in the script" },
          { key: "B", text: hook },
          { key: "C", text: "Nothing changes for me" },
        ],
        correct_key: "B",
        explanation: null,
      },
      approvals: [],
      audience: [],
      recall_pct: null,
      published_at: null,
      created_at: nowIso(),
      updated_at: nowIso(),
    };
  });
  db.reels.unshift(...reels);
  job.reel_ids = reels.map((r) => r.id);
  const topic = db.topics.find((t) => t.id === job.topic_id);
  if (topic) topic.reel_counts.draft = (topic.reel_counts.draft ?? 0) + reels.length;
}

/** Compute the job's state from elapsed wall-clock time so progress survives page reloads. */
export function materializeJob(job: GenerationJob, run: JobRun | undefined, now = Date.now()): GenerationJob {
  if (!run || job.status === "failed" || job.status === "cancelled" || job.status === "done") return job;
  const elapsed = now - run.startedAt;
  const perStep = run.stepMs;
  let changed = false;
  job.steps.forEach((step, i) => {
    if (i < run.resumeFrom) {
      if (step.status !== "done") {
        step.status = "done";
        changed = true;
      }
      return;
    }
    const idx = i - run.resumeFrom;
    const start = idx * perStep;
    const end = start + perStep;
    const next = elapsed >= end ? "done" : elapsed >= start ? "running" : "pending";
    if (step.status !== next) {
      step.status = next;
      step.attempt = Math.max(1, step.attempt ?? 0);
      step.started_at = next === "pending" ? null : new Date(run.startedAt + start).toISOString();
      step.finished_at = next === "done" ? new Date(run.startedAt + end).toISOString() : null;
      changed = true;
    }
  });
  const done = job.steps.filter((s) => s.status === "done").length;
  const running = job.steps.some((s) => s.status === "running");
  job.progress_pct = Math.min(100, Math.round(((done + (running ? 0.5 : 0)) / job.steps.length) * 100));
  const nextStatus: GenerationJob["status"] = done === job.steps.length ? "done" : elapsed > 0 ? "running" : "queued";
  if (nextStatus !== job.status) {
    job.status = nextStatus;
    changed = true;
  }
  if (job.status === "done" && !job.finished_at) {
    job.finished_at = nowIso();
    materializeReels(job);
    changed = true;
  }
  if (changed) persist();
  return job;
}

export function retryJob(job: GenerationJob): GenerationJob {
  const failedIndex = job.steps.findIndex((s) => s.status === "failed");
  if (failedIndex < 0) return job;
  job.steps.forEach((s, i) => {
    if (i >= failedIndex) {
      s.status = "pending";
      s.finished_at = null;
      s.log = null;
    }
  });
  const failed = job.steps[failedIndex];
  if (failed) failed.attempt = (failed.attempt ?? 0) + 1;
  job.status = "queued";
  job.error = null;
  db.jobRuns[job.id] = { startedAt: Date.now(), stepMs: STEP_MS, resumeFrom: failedIndex };
  persist();
  return job;
}
