"use client";

import Link from "next/link";
import { useLocale, useTranslations } from "next-intl";
import { useJobs, useSources } from "@/lib/api/hooks/studio";
import { formatDateTime, formatNumber } from "@/lib/format";
import { Badge, type BadgeTone } from "@/components/ui/badge";
import { Card, CardHeader } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";

const JOB_TONE: Record<string, BadgeTone> = { queued: "neutral", running: "ink", paused: "outline", done: "mint", failed: "coral", cancelled: "outline" };
const SOURCE_TONE: Record<string, BadgeTone> = { pending: "neutral", running: "ink", done: "mint", failed: "coral" };

export function StudioIndex() {
  const t = useTranslations("studio");
  const locale = useLocale();
  const jobs = useJobs();
  const sources = useSources();
  return (
    <>
      <PageHeader
        title={t("title")}
        description={t("subtitle")}
        actions={
          <Link href="/studio/new" className="inline-flex h-10 items-center rounded-[10px] border border-coral bg-coral px-4 font-display text-sm font-medium text-white hover:bg-coral-strong">
            {t("newSourceButton")}
          </Link>
        }
      />
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader title={t("jobs")} description={t("jobsHint")} />
          {jobs.isPending ? (
            <div className="p-5">
              <SkeletonRows rows={3} />
            </div>
          ) : jobs.isError ? (
            <div className="p-5">
              <ErrorNotice error={jobs.error} onRetry={() => void jobs.refetch()} />
            </div>
          ) : jobs.data.items.length === 0 ? (
            <div className="p-5">
              <EmptyState title={t("noJobs")} description={t("noJobsHint")} />
            </div>
          ) : (
            <ul className="divide-y divide-line">
              {jobs.data.items.map((job) => {
                const source = sources.data?.items.find((s) => s.id === job.source_id);
                return (
                  <li key={job.id} className="px-5 py-3 text-sm">
                    <Link href={`/studio/jobs/${job.id}`} className="flex items-center justify-between gap-3 hover:underline">
                      <span className="min-w-0">
                        <span className="block truncate font-medium text-ink">{source?.title ?? job.source_id}</span>
                        <span className="text-xs text-slate">
                          {formatDateTime(job.created_at, locale)} · {t("credits", { n: formatNumber(job.cost_credits, locale) })}
                        </span>
                      </span>
                      <Badge tone={JOB_TONE[job.status] ?? "neutral"}>{t(`jobStatus.${job.status}`)}</Badge>
                    </Link>
                  </li>
                );
              })}
            </ul>
          )}
        </Card>
        <Card>
          <CardHeader title={t("sources")} description={t("sourcesHint")} />
          {sources.isPending ? (
            <div className="p-5">
              <SkeletonRows rows={3} />
            </div>
          ) : sources.isError ? (
            <div className="p-5">
              <ErrorNotice error={sources.error} onRetry={() => void sources.refetch()} />
            </div>
          ) : (
            <ul className="divide-y divide-line">
              {sources.data.items.map((s) => (
                <li key={s.id} className="flex items-center justify-between gap-3 px-5 py-3 text-sm">
                  <span className="min-w-0">
                    <span className="block truncate font-medium text-ink">{s.title}</span>
                    <span className="text-xs text-slate">
                      {t(`sourceKinds.${s.kind}`)} · {formatDateTime(s.created_at, locale)}
                    </span>
                  </span>
                  <span className="flex shrink-0 items-center gap-2">
                    <Badge tone={SOURCE_TONE[s.extraction_status] ?? "neutral"}>{t(`extraction.${s.extraction_status}`)}</Badge>
                    {s.extraction_status === "done" ? (
                      <Link href={s.kind === "podcast_audio" || s.kind === "video" ? `/studio/sources/${s.id}/moments` : `/studio/jobs/new?source=${s.id}`} className="text-xs font-medium text-ink underline underline-offset-2">
                        {s.kind === "podcast_audio" || s.kind === "video" ? t("findMoments") : t("generate")}
                      </Link>
                    ) : null}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>
    </>
  );
}
