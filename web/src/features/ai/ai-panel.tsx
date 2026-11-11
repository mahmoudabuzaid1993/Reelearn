"use client";

import { useLocale, useTranslations } from "next-intl";
import { useEffect, useMemo, useState } from "react";
import { useAi, useAiUsage, useSaveAiRouting } from "@/lib/api/hooks/ai";
import type { AiRouting, AiUsage } from "@/lib/api/types";
import { formatNumber } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Select, Switch } from "@/components/ui/input";
import { SkeletonRows } from "@/components/ui/skeleton";
import { Stat } from "@/components/ui/stat";
import { Table, Td, Th, Tr } from "@/components/ui/table";
import { useToast } from "@/components/ui/toast";
import { accessState, accessTone, ALL_PROVIDERS, budgetShare, formatUsd, monthKey, type ProviderKind } from "./logic/ai";

/** Settings → AI: who writes, the Jev check, whose key each provider uses, and usage. */
export function AiPanel({ onManageKey }: { onManageKey: (kind: ProviderKind) => void }) {
  const query = useAi();
  if (query.isPending) return <SkeletonRows rows={8} />;
  if (query.isError) return <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />;
  return (
    <div className="flex flex-col gap-6">
      <RoutingCard routing={query.data.routing} />
      <AccessCard overview={query.data} onManageKey={onManageKey} />
      <UsageCard />
    </div>
  );
}

function RoutingCard({ routing }: { routing: AiRouting }) {
  const t = useTranslations("ai.routing");
  const save = useSaveAiRouting();
  const { toast } = useToast();
  const [draft, setDraft] = useState<AiRouting>(routing);
  useEffect(() => setDraft(routing), [routing]);
  const dirty = JSON.stringify(draft) !== JSON.stringify(routing);
  const set = <K extends keyof AiRouting>(k: K, v: AiRouting[K]) => setDraft((d) => ({ ...d, [k]: v }));
  const threshold = draft.review_threshold ?? 70;
  return (
    <Card>
      <CardHeader title={t("title")} description={t("hint")} />
      <CardBody className="flex flex-col gap-5">
        <Field label={t("writer")}>
          {({ id }) => (
            <Select id={id} className="max-w-sm" value={draft.writer ?? "auto"} onChange={(e) => set("writer", e.target.value as AiRouting["writer"])}>
              {(["auto", "muse", "claude"] as const).map((w) => (
                <option key={w} value={w}>
                  {t(`writers.${w}`)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Switch name="quality_check" checked={draft.quality_check ?? true} onChange={(v) => set("quality_check", v)} label={t("qualityCheck")} description={t("qualityCheckHint")} />
        <div className="flex flex-col gap-1.5">
          <label htmlFor="review-threshold" className="flex items-center justify-between text-sm text-ink">
            <span>{t("threshold")}</span>
            <span className="tabular-nums font-medium">{threshold}</span>
          </label>
          <input
            id="review-threshold"
            type="range"
            min={0}
            max={100}
            step={5}
            value={threshold}
            disabled={!(draft.quality_check ?? true)}
            aria-describedby="review-threshold-hint"
            onChange={(e) => set("review_threshold", Number(e.target.value))}
            className="w-full accent-ink"
          />
          <p id="review-threshold-hint" className="text-xs text-slate">
            {t("thresholdHint")}
          </p>
        </div>
        <Switch name="role_matching" checked={draft.role_matching ?? true} onChange={(v) => set("role_matching", v)} label={t("roleMatching")} description={t("roleMatchingHint")} />
        <Switch name="tag_check" checked={draft.tag_check ?? true} onChange={(v) => set("tag_check", v)} label={t("tagCheck")} description={t("tagCheckHint")} />
        <div className="flex items-center gap-3">
          <Button
            variant="primary"
            disabled={!dirty}
            loading={save.isPending}
            onClick={() => save.mutate(draft, { onSuccess: () => toast({ title: t("saved"), tone: "success" }) })}
          >
            {t("save")}
          </Button>
          {save.error ? <ErrorNotice error={save.error} /> : null}
        </div>
      </CardBody>
    </Card>
  );
}

function AccessCard({ overview, onManageKey }: { overview: NonNullable<ReturnType<typeof useAi>["data"]>; onManageKey: (kind: ProviderKind) => void }) {
  const t = useTranslations("ai");
  const locale = useLocale();
  const { plan } = overview;
  const share = budgetShare(plan.spent_this_month_usd, plan.monthly_budget_usd);
  return (
    <Card>
      <CardHeader title={t("access.title")} description={t("access.hint")} />
      <CardBody className="flex flex-col gap-4">
        <div className="flex flex-col gap-2 rounded-[10px] border border-line p-3" data-testid="ai-plan">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="font-medium text-ink">{t("plan.title")}</span>
            <Badge tone={plan.platform_keys ? "mint" : "neutral"}>{plan.platform_keys ? t("plan.platformKeys") : t("plan.noPlatformKeys")}</Badge>
          </div>
          {plan.platform_keys ? (
            <>
              <p className="text-sm text-slate">
                {plan.monthly_budget_usd === null || plan.monthly_budget_usd === undefined
                  ? t("plan.unlimited", { spent: formatUsd(plan.spent_this_month_usd, locale) })
                  : t("plan.budget", { spent: formatUsd(plan.spent_this_month_usd, locale), budget: formatUsd(plan.monthly_budget_usd, locale) })}
              </p>
              {share !== null ? (
                <div role="meter" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(share * 100)} aria-label={t("plan.title")} className="h-2 overflow-hidden rounded-full bg-surface-muted">
                  <div className="h-full bg-ink" style={{ width: `${Math.round(share * 100)}%` }} />
                </div>
              ) : null}
              <p className="text-xs text-slate">{plan.allowed.length === 0 ? t("plan.allowedAll") : t("plan.allowedSome", { list: plan.allowed.map((k) => t(`providers.${k}` as never)).join(" · ") })}</p>
            </>
          ) : null}
        </div>
        <ul className="divide-y divide-line">
          {ALL_PROVIDERS.map((kind) => {
            const p = overview.providers.find((x) => x.kind === kind);
            const state = accessState(p);
            return (
              <li key={kind} className="flex flex-wrap items-center justify-between gap-3 py-3" data-testid={`access-${kind}`}>
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-medium text-ink">{t(`providers.${kind}`)}</p>
                    <Badge tone={accessTone(state)}>{t(`access.states.${state}`)}</Badge>
                  </div>
                  <p className="mt-0.5 text-xs text-slate">
                    {t(`roles.${kind}`)}
                    {p?.model ? ` · ${t("access.model", { model: p.model })}` : ""}
                  </p>
                </div>
                <Button size="sm" variant="secondary" onClick={() => onManageKey(kind)}>
                  {p?.company_key ? t("access.manageKey") : t("access.addKey")}
                </Button>
              </li>
            );
          })}
        </ul>
      </CardBody>
    </Card>
  );
}

function UsageCard() {
  const t = useTranslations("ai.usage");
  const locale = useLocale();
  const months = useMemo(() => [0, -1, -2].map((d) => monthKey(new Date(), d)), []);
  const [month, setMonth] = useState(months[0]!);
  const query = useAiUsage(month);
  return (
    <Card>
      <CardHeader
        title={t("title")}
        description={t("hint")}
        actions={
          <Select aria-label={t("month")} value={month} onChange={(e) => setMonth(e.target.value)} className="w-32" dir="ltr">
            {months.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </Select>
        }
      />
      <CardBody>
        {query.isPending ? (
          <SkeletonRows rows={4} />
        ) : query.isError ? (
          <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />
        ) : query.data.lines.length === 0 ? (
          <EmptyState title={t("empty")} />
        ) : (
          <UsageBody usage={query.data} locale={locale} />
        )}
      </CardBody>
    </Card>
  );
}

function UsageBody({ usage, locale }: { usage: AiUsage; locale: string }) {
  const t = useTranslations("ai");
  const calls = usage.lines.reduce((n, l) => n + l.calls, 0);
  const failed = usage.lines.reduce((n, l) => n + l.failed, 0);
  const max = Math.max(1, ...usage.days.map((d) => d.calls));
  return (
    <div className="flex flex-col gap-5">
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <Stat label={t("usage.calls")} value={formatNumber(calls, locale)} hint={failed ? `${t("usage.failed")}: ${formatNumber(failed, locale)}` : undefined} />
        <Stat label={t("usage.platformBill")} value={formatUsd(usage.platform_billable_usd, locale)} />
        <Stat label={t("usage.companyCost")} value={formatUsd(usage.company_cost_usd, locale)} />
      </div>
      <figure>
        <figcaption className="mb-2 text-xs font-medium uppercase tracking-wide text-slate">{t("usage.daily")}</figcaption>
        <div aria-hidden className="flex h-20 items-end gap-1">
          {usage.days.map((d) => (
            <div key={d.day} className="flex-1 rounded-t bg-ink/80" style={{ height: `${Math.max(4, (d.calls / max) * 100)}%` }} title={`${d.day}: ${d.calls}`} />
          ))}
        </div>
        <table className="sr-only">
          <caption>{t("usage.dailyTable")}</caption>
          <tbody>
            {usage.days.map((d) => (
              <tr key={d.day}>
                <th scope="row">{d.day}</th>
                <td>{d.calls}</td>
                <td>{formatUsd(d.cost_usd ?? null, locale)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </figure>
      <Table>
        <thead>
          <Tr>
            <Th>{t("usage.provider")}</Th>
            <Th>{t("usage.capability")}</Th>
            <Th>{t("usage.source")}</Th>
            <Th className="text-end">{t("usage.calls")}</Th>
            <Th className="text-end">{t("usage.tokens")}</Th>
            <Th className="text-end">{t("usage.cost")}</Th>
          </Tr>
        </thead>
        <tbody>
          {usage.lines.map((l) => (
            <Tr key={`${l.provider}-${l.capability}-${l.key_source}`}>
              <Td>{t.has(`providers.${l.provider}`) ? t(`providers.${l.provider}` as never) : l.provider}</Td>
              <Td>{t(`usage.capabilities.${l.capability}`)}</Td>
              <Td>
                <Badge tone={l.key_source === "company" ? "outline" : "ink"}>{t(`usage.sources.${l.key_source}`)}</Badge>
              </Td>
              <Td className="text-end tabular-nums">{formatNumber(l.calls, locale)}</Td>
              <Td className="text-end tabular-nums" dir="ltr">
                {formatNumber(l.input_tokens, locale)} / {formatNumber(l.output_tokens, locale)}
              </Td>
              <Td className="text-end tabular-nums">{formatUsd(l.cost_usd ?? null, locale)}</Td>
            </Tr>
          ))}
        </tbody>
      </Table>
    </div>
  );
}
