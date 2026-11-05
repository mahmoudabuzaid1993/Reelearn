"use client";

import Link from "next/link";
import { useLocale, useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { useRetryJob, useSource } from "@/lib/api/hooks/studio";
import { useJobProgress } from "@/lib/api/hooks/use-job-progress";
import { API_BASE, authHeaders } from "@/lib/api/client";
import { formatDateTime, formatNumber } from "@/lib/format";
import { Badge, type BadgeTone } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Check } from "@/components/ui/icons";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { canOpenStoryboard, failedStep } from "./logic/job-status";

const STEP_TONE: Record<string, BadgeTone> = { pending: "neutral", running: "ink", done: "mint", failed: "coral", paused: "outline" };

export function JobProgress({ jobId }: { jobId: string }) {
  const t = useTranslations("studio.progress");
  const locale = useLocale();
  const router = useRouter();
  const [epoch, setEpoch] = useState(0);
  const { job, mode, error } = useJobProgress(jobId, { baseUrl: API_BASE, headers: authHeaders, epoch });
  const source = useSource(job?.source_id ?? "");
  const retry = useRetryJob();

  useEffect(() => {
    if (job && canOpenStoryboard(job)) {
      const handle = setTimeout(() => router.push(`/studio/jobs/${job.id}/storyboard`), 1200);
      return () => clearTimeout(handle);
    }
  }, [job, router]);

  if (!job) {
    return (
      <>
        <PageHeader eyebrow={t("step")} title={t("title")} />
        {error ? <ErrorNotice error={new Error(error)} /> : <SkeletonRows rows={6} />}
      </>
    );
  }

  const failed = failedStep(job);
  return (
    <>
      <PageHeader
        eyebrow={t("step")}
        title={t("title")}
        description={source.data ? source.data.title : undefined}
        actions={
          <Badge tone="outline" aria-live="polite">
            {mode === "sse" ? t("live") : mode === "polling" ? t("polling") : t("connecting")}
          </Badge>
        }
      />
      <Card>
        <CardHeader
          title={t(`status.${job.status}`)}
          description={t("costLine", { n: formatNumber(job.cost_credits, locale), at: formatDateTime(job.created_at, locale) })}
          actions={<span className="font-display text-xl font-semibold tabular-nums text-ink">{job.progress_pct}%</span>}
        />
        <CardBody className="flex flex-col gap-4">
          <div role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={job.progress_pct} aria-label={t("title")} className="h-2 overflow-hidden rounded-full bg-surface-muted">
            <div className="h-full rounded-full bg-mint transition-[width]" style={{ width: `${job.progress_pct}%` }} />
          </div>
          <ol className="flex flex-col gap-2">
            {job.steps.map((step) => (
              <li key={step.position} className="flex items-center justify-between gap-3 rounded-[10px] border border-line px-3 py-2 text-sm">
                <span className="flex items-center gap-2">
                  <span aria-hidden className={`inline-flex size-5 items-center justify-center rounded-full ${step.status === "done" ? "bg-mint text-white" : "bg-surface-muted text-slate"}`}>
                    {step.status === "done" ? <Check width={12} height={12} /> : step.position}
                  </span>
                  <span className="font-medium text-ink">{t(`steps.${step.step}`)}</span>
                  {step.provider ? <span className="text-xs text-slate">· {step.provider}</span> : null}
                </span>
                <span className="flex items-center gap-2">
                  {step.log ? <span className="max-w-xs truncate text-xs text-slate">{step.log}</span> : null}
                  <Badge tone={STEP_TONE[step.status] ?? "neutral"}>{t(`stepStatus.${step.status}`)}</Badge>
                </span>
              </li>
            ))}
          </ol>

          {job.status === "paused" ? <p className="text-sm text-slate">{t("pausedHint")}</p> : null}
          {job.status === "failed" && failed ? (
            <div role="alert" className="flex flex-wrap items-center justify-between gap-3 rounded-[10px] border border-ink/30 px-4 py-3 text-sm">
              <span>
                <span className="font-medium text-ink">{t("failedAt", { step: t(`steps.${failed.step}`) })}</span>
                {job.error ? <span className="block text-xs text-slate">{job.error}</span> : null}
              </span>
              <Button variant="primary" loading={retry.isPending} onClick={() => retry.mutate(job.id, { onSuccess: () => setEpoch((e) => e + 1) })}>
                {t("retryStep")}
              </Button>
            </div>
          ) : null}
          {retry.error ? <ErrorNotice error={retry.error} /> : null}

          {canOpenStoryboard(job) ? (
            <div className="flex items-center justify-between gap-3 rounded-[10px] border border-mint bg-mint-soft px-4 py-3 text-sm">
              <span className="font-medium text-ink">{t("done", { n: job.reel_ids?.length ?? 0 })}</span>
              <Link href={`/studio/jobs/${job.id}/storyboard`} className="inline-flex h-9 items-center rounded-[10px] border border-ink bg-ink px-3 font-display text-sm font-medium text-white hover:bg-night">
                {t("openStoryboard")}
              </Link>
            </div>
          ) : null}
        </CardBody>
      </Card>
    </>
  );
}
