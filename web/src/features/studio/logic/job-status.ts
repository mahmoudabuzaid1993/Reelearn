import type { GenerationJob, GenerationStep } from "@/lib/api/types";

export const JOB_STEPS = ["extract", "script", "question", "render", "captions", "dub"] as const;

export function failedStep(job: Pick<GenerationJob, "steps">): GenerationStep | null {
  return job.steps.find((s) => s.status === "failed") ?? null;
}

export function isTerminal(status: GenerationJob["status"]): boolean {
  return status === "done" || status === "failed" || status === "cancelled";
}

export function canOpenStoryboard(job: Pick<GenerationJob, "status" | "reel_ids">): boolean {
  return job.status === "done" && (job.reel_ids?.length ?? 0) > 0;
}

export function progressFromSteps(steps: GenerationStep[]): number {
  if (steps.length === 0) return 0;
  const done = steps.filter((s) => s.status === "done").length;
  const running = steps.some((s) => s.status === "running") ? 0.5 : 0;
  return Math.min(100, Math.round(((done + running) / steps.length) * 100));
}
