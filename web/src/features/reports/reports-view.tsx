"use client";

import { useLocale, useTranslations } from "next-intl";
import { useState } from "react";
import { useCreateExport, useDownloadExport, useExports, useReportByGroup, useReportByTopic, useReportOverview, type Range } from "@/lib/api/hooks/reports";
import type { ReportRow } from "@/lib/api/types";
import { formatDate, formatDateTime, formatNumber, formatPct } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton, SkeletonRows } from "@/components/ui/skeleton";
import { Stat } from "@/components/ui/stat";
import { Table, Td, Th, Tr } from "@/components/ui/table";
import { Tabs } from "@/components/ui/tabs";
import { useToast } from "@/components/ui/toast";

type Tab = "topic" | "group" | "hrdf";

export function ReportsView() {
  const t = useTranslations("reports");
  const locale = useLocale();
  const [range, setRange] = useState<Range>("30d");
  const [tab, setTab] = useState<Tab>("topic");
  const overview = useReportOverview(range);

  return (
    <>
      <PageHeader
        title={t("title")}
        description={t("subtitle")}
        actions={
          <div role="group" aria-label={t("range")} className="flex rounded-[10px] border border-line bg-surface p-0.5">
            {(["7d", "30d", "90d", "12m"] as Range[]).map((r) => (
              <button key={r} type="button" aria-pressed={range === r} onClick={() => setRange(r)} className={`h-8 rounded-lg px-3 text-[13px] font-medium ${range === r ? "bg-ink text-white" : "text-slate hover:text-ink"}`}>
                {t(`ranges.${r}`)}
              </button>
            ))}
          </div>
        }
      />
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-5">
        {overview.isPending ? (
          Array.from({ length: 5 }, (_, i) => (
            <Card key={i} className="px-5 py-4">
              <Skeleton className="h-3 w-20" />
              <Skeleton className="mt-3 h-7 w-16" />
            </Card>
          ))
        ) : overview.data ? (
          <>
            <Stat label={t("activeLearners")} value={formatNumber(overview.data.learners_active, locale)} />
            <Stat label={t("reelsCompleted")} value={formatNumber(overview.data.reels_completed, locale)} />
            <Stat label={t("answers")} value={formatNumber(overview.data.answers_total, locale)} />
            <Stat label={t("recall")} value={formatPct(overview.data.recall_pct, locale)} />
            <Stat label={t("mandatoryMinutes")} value={formatNumber(overview.data.minutes_on_mandatory, locale)} hint={t("mandatoryHint")} />
          </>
        ) : null}
      </div>
      {overview.isError ? <div className="mt-4"><ErrorNotice error={overview.error} onRetry={() => void overview.refetch()} /></div> : null}

      <div className="mt-6">
        <Tabs ariaLabel={t("title")} value={tab} onChange={setTab} items={[{ value: "topic", label: t("byTopic") }, { value: "group", label: t("byGroup") }, { value: "hrdf", label: t("hrdf") }]} />
        <div className="mt-4">
          {tab === "topic" ? <RowsTable kind="topic" range={range} /> : null}
          {tab === "group" ? <RowsTable kind="group" range={range} /> : null}
          {tab === "hrdf" ? <HrdfPanel /> : null}
        </div>
      </div>
      <p className="mt-6 text-xs text-slate">{t("privacyNote")}</p>
    </>
  );
}

function RowsTable({ kind, range }: { kind: "topic" | "group"; range: Range }) {
  const t = useTranslations("reports");
  const locale = useLocale();
  const byTopic = useReportByTopic(range);
  const byGroup = useReportByGroup(range);
  const query = kind === "topic" ? byTopic : byGroup;
  if (query.isPending) return <SkeletonRows rows={4} />;
  if (query.isError) return <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />;
  const rows: ReportRow[] = query.data;
  if (rows.length === 0) return <EmptyState title={t("noRows")} />;
  return (
    <Table>
      <thead>
        <tr>
          <Th>{kind === "topic" ? t("topic") : t("group")}</Th>
          <Th>{t("activeLearners")}</Th>
          <Th>{t("recall")}</Th>
          <Th>{t("ready")}</Th>
          <Th>{t("reelsCompleted")}</Th>
          <Th>{t("minutes")}</Th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r) => (
          <Tr key={r.id}>
            <Td className="font-medium">{r.name}</Td>
            <Td>
              {formatNumber(r.learners_active, locale)}
              {r.learners_total !== undefined ? <span className="text-slate"> / {formatNumber(r.learners_total, locale)}</span> : null}
            </Td>
            <Td>
              <RecallBar value={r.recall_pct} />
            </Td>
            <Td>{formatPct(r.ready_pct, locale)}</Td>
            <Td>{formatNumber(r.reels_completed, locale)}</Td>
            <Td>{formatNumber(r.minutes_total, locale, 1)}</Td>
          </Tr>
        ))}
      </tbody>
    </Table>
  );
}

export function RecallBar({ value }: { value: number | null | undefined }) {
  const locale = useLocale();
  if (value === null || value === undefined) return <span className="text-slate">—</span>;
  return (
    <span className="inline-flex items-center gap-2">
      <span aria-hidden className="h-1.5 w-20 overflow-hidden rounded-full bg-surface-muted">
        <span className="block h-full rounded-full bg-mint" style={{ width: `${Math.max(0, Math.min(100, value))}%` }} />
      </span>
      <span className="tabular-nums">{formatPct(value, locale)}</span>
    </span>
  );
}

function HrdfPanel() {
  const t = useTranslations("reports");
  const locale = useLocale();
  const { toast } = useToast();
  const exportsQuery = useExports(4000);
  const create = useCreateExport();
  const download = useDownloadExport();
  const [period, setPeriod] = useState({ start: "2026-10-01", end: "2026-12-31" });

  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
      <Card>
        <CardHeader title={t("evidencePack")} description={t("evidenceHint")} />
        <CardBody className="flex flex-col gap-3">
          <Field label={t("periodStart")}>{({ id }) => <Input id={id} type="date" dir="ltr" value={period.start} onChange={(e) => setPeriod((p) => ({ ...p, start: e.target.value }))} />}</Field>
          <Field label={t("periodEnd")}>{({ id }) => <Input id={id} type="date" dir="ltr" value={period.end} onChange={(e) => setPeriod((p) => ({ ...p, end: e.target.value }))} />}</Field>
          {create.error ? <ErrorNotice error={create.error} /> : null}
          <Button
            variant="primary"
            loading={create.isPending}
            disabled={!period.start || !period.end || period.end < period.start}
            onClick={() => create.mutate({ kind: "hrdf_evidence", period_start: period.start, period_end: period.end }, { onSuccess: () => toast({ title: t("exportQueued"), tone: "success" }) })}
          >
            {t("buildPack")}
          </Button>
        </CardBody>
      </Card>
      <Card className="lg:col-span-2">
        <CardHeader title={t("exports")} />
        {exportsQuery.isPending ? (
          <CardBody>
            <SkeletonRows rows={3} />
          </CardBody>
        ) : exportsQuery.isError ? (
          <CardBody>
            <ErrorNotice error={exportsQuery.error} />
          </CardBody>
        ) : exportsQuery.data.length === 0 ? (
          <CardBody>
            <EmptyState title={t("noExports")} description={t("noExportsHint")} />
          </CardBody>
        ) : (
          <ul className="divide-y divide-line">
            {exportsQuery.data.map((e) => (
              <li key={e.id} className="flex flex-wrap items-center justify-between gap-3 px-5 py-3 text-sm">
                <div>
                  <p className="font-medium text-ink">
                    {t("kinds.hrdf_evidence")} · {formatDate(e.period_start, locale)} – {formatDate(e.period_end, locale)}
                  </p>
                  <p className="text-xs text-slate">{formatDateTime(e.created_at, locale)}</p>
                </div>
                <div className="flex items-center gap-2">
                  <Badge tone={e.status === "ready" ? "mint" : "neutral"}>{t(`exportStatus.${e.status}`)}</Badge>
                  {e.status === "ready" ? (
                    <Button
                      size="sm"
                      loading={download.isPending && download.variables === e.id}
                      onClick={() => download.mutate(e.id, { onSuccess: (r) => toast({ title: t("downloadReady"), description: r.url }) })}
                    >
                      {t("download")}
                    </Button>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  );
}
