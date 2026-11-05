"use client";

import Link from "next/link";
import { useLocale, useTranslations } from "next-intl";
import { useState } from "react";
import { useDashboard } from "@/lib/api/hooks/tenant";
import { formatNumber, formatPct } from "@/lib/format";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { ArrowEnd } from "@/components/ui/icons";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { Stat } from "@/components/ui/stat";
import { ActivityChart } from "./activity-chart";

type Range = "7d" | "30d" | "90d" | "12m";

export function DashboardView() {
  const t = useTranslations("dashboard");
  const locale = useLocale();
  const [range, setRange] = useState<Range>("30d");
  const query = useDashboard(range);

  return (
    <>
      <PageHeader
        title={t("title")}
        description={t("subtitle")}
        actions={
          <div role="group" aria-label={t("range")} className="flex rounded-[10px] border border-line bg-surface p-0.5">
            {(["7d", "30d", "90d", "12m"] as Range[]).map((r) => (
              <button
                key={r}
                type="button"
                aria-pressed={range === r}
                onClick={() => setRange(r)}
                className={`h-8 rounded-lg px-3 text-[13px] font-medium ${range === r ? "bg-ink text-white" : "text-slate hover:text-ink"}`}
              >
                {t(`ranges.${r}`)}
              </button>
            ))}
          </div>
        }
      />

      {query.isError ? <ErrorNotice error={query.error} onRetry={() => void query.refetch()} /> : null}

      <div className="grid grid-cols-2 gap-4 lg:grid-cols-5">
        {query.isPending
          ? Array.from({ length: 5 }, (_, i) => (
              <Card key={i} className="px-5 py-4">
                <Skeleton className="h-3 w-20" />
                <Skeleton className="mt-3 h-7 w-16" />
              </Card>
            ))
          : query.data
            ? (
                <>
                  <Stat label={t("weeklyActive")} value={formatPct(query.data.weekly_active_pct, locale)} hint={t("ofLearners", { active: query.data.learners_active ?? 0, total: query.data.learners_total ?? 0 })} />
                  <Stat label={t("recall")} value={formatPct(query.data.recall_pct, locale)} hint={t("recallHint")} />
                  <Stat label={t("learners")} value={formatNumber(query.data.learners_total, locale)} hint={t("learnersHint", { active: query.data.learners_active ?? 0 })} />
                  <Stat label={t("hrdfHours")} value={formatNumber(query.data.hrdf_hours, locale, 1)} hint={t("hrdfHint")} />
                  <Stat label={t("credits")} value={formatNumber(query.data.credits, locale)} hint={<Link href="/settings#credits" className="underline underline-offset-2">{t("manageCredits")}</Link>} />
                </>
              )
            : null}
      </div>

      <div className="mt-6 grid grid-cols-1 gap-4 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <CardHeader title={t("activity")} description={t("activityHint")} />
          <CardBody>
            {query.isPending ? <Skeleton className="h-48" /> : query.data?.series?.length ? <ActivityChart series={query.data.series} /> : <EmptyState title={t("noActivity")} />}
          </CardBody>
        </Card>
        <Card>
          <CardHeader title={t("attention")} description={t("attentionHint")} />
          <CardBody className="flex flex-col gap-2">
            {query.isPending ? (
              <>
                <Skeleton /> <Skeleton className="w-3/4" /> <Skeleton className="w-1/2" />
              </>
            ) : query.data?.attention?.length ? (
              query.data.attention.map((item, i) => (
                <Link key={i} href={item.href ?? "#"} className="flex items-center justify-between gap-3 rounded-[10px] border border-line px-3 py-2.5 text-sm text-ink hover:bg-paper">
                  <span>{item.text}</span>
                  <ArrowEnd className="shrink-0 text-slate" />
                </Link>
              ))
            ) : (
              <EmptyState title={t("nothingNeedsAttention")} />
            )}
          </CardBody>
        </Card>
      </div>

      <div className="mt-6 flex flex-wrap gap-2">
        <Link href="/studio/new" className="inline-flex h-10 items-center rounded-[10px] border border-coral bg-coral px-4 font-display text-sm font-medium text-white hover:bg-coral-strong">
          {t("makeReels")}
        </Link>
        <Link href="/library?status=in_review" className="inline-flex h-10 items-center rounded-[10px] border border-line bg-surface px-4 font-display text-sm font-medium text-ink hover:bg-surface-muted">
          {t("reviewQueue")}
        </Link>
      </div>
    </>
  );
}
