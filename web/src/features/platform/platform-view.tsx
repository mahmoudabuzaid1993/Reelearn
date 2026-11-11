"use client";

import { useLocale, useTranslations } from "next-intl";
import { useMemo, useState } from "react";
import {
  downloadUsageCsv,
  usePlatformIntegrations,
  usePlatformMe,
  usePlatformTenants,
  useSaveAiPlan,
  useSavePlatformIntegration,
  useTestPlatformIntegration,
} from "@/lib/api/hooks/ai";
import type { PlatformIntegration, TenantAi } from "@/lib/api/types";
import { formatNumber } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Checkbox, Input, Select, Switch } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { Table, Td, Th, Tr } from "@/components/ui/table";
import { Tabs } from "@/components/ui/tabs";
import { useToast } from "@/components/ui/toast";
import {
  ALL_PROVIDERS,
  budgetShare,
  configPatch,
  draftFrom,
  formatUsd,
  monthKey,
  type PlanDraft,
  planFromDraft,
  PRICE_FIELDS,
  PROVIDER_FIELDS,
  type ProviderKind,
} from "@/features/ai/logic/ai";

type Tab = "providers" | "companies";

/** For the people who run Reelearn: platform keys and prices, AI plans, invoice export. */
export function PlatformView() {
  const t = useTranslations("platform");
  const me = usePlatformMe();
  const [tab, setTab] = useState<Tab>("providers");
  if (me.isPending) return <SkeletonRows rows={6} />;
  if (me.isError) return <ErrorNotice error={me.error} onRetry={() => void me.refetch()} />;
  if (!me.data.operator) return <EmptyState title={t("forbidden")} />;
  return (
    <>
      <PageHeader title={t("title")} description={t("subtitle")} />
      <Tabs
        ariaLabel={t("title")}
        value={tab}
        onChange={setTab}
        items={[
          { value: "providers", label: t("tabs.providers") },
          { value: "companies", label: t("tabs.companies") },
        ]}
      />
      <div className="mt-6">{tab === "providers" ? <ProvidersPanel /> : <CompaniesPanel />}</div>
    </>
  );
}

// ---------------------------------------------------------------- platform keys

function ProvidersPanel() {
  const t = useTranslations("platform.providers");
  const ta = useTranslations("ai");
  const ts = useTranslations("settings");
  const query = usePlatformIntegrations();
  const test = useTestPlatformIntegration();
  const { toast } = useToast();
  const [editing, setEditing] = useState<ProviderKind | null>(null);
  if (query.isPending) return <SkeletonRows rows={7} />;
  if (query.isError) return <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />;
  return (
    <Card>
      <CardHeader title={t("title")} description={t("hint")} />
      <ul className="divide-y divide-line">
        {ALL_PROVIDERS.map((kind) => {
          const row: PlatformIntegration = query.data.find((r) => r.kind === kind) ?? { kind, status: "disconnected", public_config: {} };
          const tone = row.status === "connected" ? "mint" : row.status === "error" ? "coral" : "neutral";
          const config = Object.entries(row.public_config ?? {})
            .filter(([k]) => k !== "api_key" && k !== "access_token")
            .map(([k, v]) => `${ts.has(`fields.${k}`) ? ts(`fields.${k}` as never) : k}: ${String(v)}`)
            .join(" · ");
          return (
            <li key={kind} className="flex flex-wrap items-center justify-between gap-3 px-5 py-3" data-testid={`platform-${kind}`}>
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <p className="font-medium text-ink">{ta(`providers.${kind}`)}</p>
                  <Badge tone={tone}>{ts(`status.${row.status}`)}</Badge>
                  {row.source ? <Badge tone="outline">{t(`source.${row.source}`)}</Badge> : null}
                </div>
                <p className="mt-0.5 truncate text-xs text-slate">{row.last_error ?? (config || ta(`roles.${kind}`))}</p>
              </div>
              <div className="flex gap-2">
                <Button
                  size="sm"
                  loading={test.isPending && test.variables === kind}
                  onClick={() =>
                    test.mutate(kind, {
                      onSuccess: (r) => {
                        toast({ title: r.ok ? t("testOk") : t("testFailed"), description: r.message, tone: r.ok ? "success" : "neutral" });
                        void query.refetch();
                      },
                    })
                  }
                >
                  {t("test")}
                </Button>
                <Button size="sm" variant="secondary" onClick={() => setEditing(kind)}>
                  {t("configure")}
                </Button>
              </div>
            </li>
          );
        })}
      </ul>
      <PlatformKeyDialog kind={editing} onClose={() => setEditing(null)} />
    </Card>
  );
}

function PlatformKeyDialog({ kind, onClose }: { kind: ProviderKind | null; onClose: () => void }) {
  const t = useTranslations("platform.providers");
  const ta = useTranslations("ai");
  const ts = useTranslations("settings");
  const save = useSavePlatformIntegration();
  const { toast } = useToast();
  const [values, setValues] = useState<Record<string, string>>({});
  const [touched, setTouched] = useState<Set<string>>(new Set());
  if (!kind) return null;
  const close = () => {
    setValues({});
    setTouched(new Set());
    onClose();
  };
  const prices = kind === "whatsapp" ? [] : PRICE_FIELDS.filter((p) => kind !== "ai_typesafe" || p !== "price_per_call");
  const field = (name: string, secret?: boolean, optional?: boolean) => (
    <Field key={name} label={optional ? `${ts(`fields.${name}` as never)} (${ts("fields.optional")})` : ts(`fields.${name}` as never)}>
      {({ id }) => (
        <Input
          id={id}
          dir="ltr"
          type={secret ? "password" : "text"}
          inputMode={name.startsWith("price_") ? "decimal" : undefined}
          autoComplete="off"
          value={values[name] ?? ""}
          onChange={(e) => {
            setValues((v) => ({ ...v, [name]: e.target.value }));
            setTouched((s) => new Set(s).add(name));
          }}
        />
      )}
    </Field>
  );
  return (
    <Dialog
      open
      onClose={close}
      title={t("dialogTitle", { kind: ta(`providers.${kind}`) })}
      description={t("dialogHint")}
      footer={
        <>
          <Button onClick={close}>{ts("cancel")}</Button>
          <Button
            variant="primary"
            loading={save.isPending}
            onClick={() =>
              save.mutate(
                { kind, body: configPatch(values, touched) },
                {
                  onSuccess: () => {
                    toast({ title: t("saved"), tone: "success" });
                    close();
                  },
                },
              )
            }
          >
            {ts("save")}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-3">
        {PROVIDER_FIELDS[kind].map((f) => field(f.name, f.secret, f.optional))}
        {prices.length ? (
          <fieldset className="flex flex-col gap-3 border-t border-line pt-3">
            <legend className="text-[13px] font-medium text-ink">{t("prices")}</legend>
            {prices.map((p) => field(p, false, true))}
          </fieldset>
        ) : null}
        {save.error ? <ErrorNotice error={save.error} /> : null}
      </div>
    </Dialog>
  );
}

// ---------------------------------------------------------------- companies and plans

function CompaniesPanel() {
  const t = useTranslations("platform.companies");
  const locale = useLocale();
  const { toast } = useToast();
  const months = useMemo(() => [0, -1, -2].map((d) => monthKey(new Date(), d)), []);
  const [month, setMonth] = useState(months[0]!);
  const query = usePlatformTenants(month);
  const [editing, setEditing] = useState<TenantAi | null>(null);
  const [downloading, setDownloading] = useState(false);

  const download = async () => {
    setDownloading(true);
    try {
      const blob = await downloadUsageCsv(month);
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `reelearn-ai-usage-${month}.csv`;
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      toast({ title: t("export"), description: "—", tone: "neutral" });
    } finally {
      setDownloading(false);
    }
  };

  return (
    <Card>
      <CardHeader
        title={t("title")}
        description={t("hint")}
        actions={
          <>
            <Select aria-label="Month" value={month} onChange={(e) => setMonth(e.target.value)} className="w-32" dir="ltr">
              {months.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </Select>
            <Button size="sm" className="whitespace-nowrap" loading={downloading} onClick={() => void download()}>
              {t("export")}
            </Button>
          </>
        }
      />
      <CardBody>
        {query.isPending ? (
          <SkeletonRows rows={4} />
        ) : query.isError ? (
          <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />
        ) : query.data.tenants.length === 0 ? (
          <EmptyState title={t("empty")} />
        ) : (
          <Table>
            <thead>
              <Tr>
                <Th>{t("company")}</Th>
                <Th>{t("plan")}</Th>
                <Th className="text-end">{t("calls")}</Th>
                <Th className="text-end">{t("platformCost")}</Th>
                <Th className="text-end">{t("billable")}</Th>
                <Th className="text-end">{t("companyCost")}</Th>
                <Th>{t("budget")}</Th>
                <Th />
              </Tr>
            </thead>
            <tbody>
              {query.data.tenants.map((r) => {
                const share = r.budget_used ?? budgetShare(r.billable_usd, r.plan.monthly_budget_usd);
                return (
                  <Tr key={r.id} data-testid={`tenant-${r.slug}`}>
                    <Td>
                      <p className="font-medium text-ink">{r.name}</p>
                      <p className="text-xs text-slate" dir="ltr">
                        {r.slug}
                      </p>
                    </Td>
                    <Td>
                      <Badge tone={r.plan.platform_keys ? "mint" : "neutral"}>
                        {r.plan.platform_keys ? `+${formatNumber(r.plan.markup_pct ?? 0, locale)}%` : t("noPlatformKeys")}
                      </Badge>
                    </Td>
                    <Td className="text-end tabular-nums">{formatNumber(r.calls, locale)}</Td>
                    <Td className="text-end tabular-nums">{formatUsd(r.platform_cost_usd, locale)}</Td>
                    <Td className="text-end font-medium tabular-nums">{formatUsd(r.billable_usd, locale)}</Td>
                    <Td className="text-end tabular-nums">
                      {formatUsd(r.company_cost_usd, locale)}
                      {r.unpriced_calls ? <p className="text-xs text-slate">{t("unpriced", { n: r.unpriced_calls })}</p> : null}
                    </Td>
                    <Td className="min-w-32">
                      {share === null ? (
                        <span className="text-xs text-slate">{t("unlimited")}</span>
                      ) : (
                        <div className="flex flex-col gap-1">
                          <div role="meter" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(share * 100)} aria-label={`${t("budget")} ${r.name}`} className="h-2 overflow-hidden rounded-full bg-surface-muted">
                            <div className="h-full bg-ink" style={{ width: `${Math.round(share * 100)}%` }} />
                          </div>
                          <span className="text-xs text-slate tabular-nums">{formatUsd(r.plan.monthly_budget_usd ?? null, locale)}</span>
                        </div>
                      )}
                    </Td>
                    <Td>
                      <Button size="sm" variant="secondary" onClick={() => setEditing(r)}>
                        {t("edit")}
                      </Button>
                    </Td>
                  </Tr>
                );
              })}
            </tbody>
          </Table>
        )}
      </CardBody>
      {editing ? <PlanDialog tenant={editing} onClose={() => setEditing(null)} /> : null}
    </Card>
  );
}

function PlanDialog({ tenant, onClose }: { tenant: TenantAi; onClose: () => void }) {
  const t = useTranslations("platform.planDialog");
  const ta = useTranslations("ai");
  const ts = useTranslations("settings");
  const save = useSaveAiPlan();
  const { toast } = useToast();
  const [draft, setDraft] = useState<PlanDraft>(() => draftFrom(tenant.plan));
  const [error, setError] = useState<"budget" | "markup" | null>(null);
  const submit = () => {
    const parsed = planFromDraft(draft);
    if ("error" in parsed) {
      setError(parsed.error);
      return;
    }
    setError(null);
    save.mutate(
      { tenantId: tenant.id, plan: parsed.plan },
      {
        onSuccess: () => {
          toast({ title: t("saved"), tone: "success" });
          onClose();
        },
      },
    );
  };
  return (
    <Dialog
      open
      onClose={onClose}
      title={t("title", { name: tenant.name })}
      footer={
        <>
          <Button onClick={onClose}>{ts("cancel")}</Button>
          <Button variant="primary" loading={save.isPending} onClick={submit}>
            {ts("save")}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <Switch name="platform_keys" checked={draft.platform_keys} onChange={(v) => setDraft((d) => ({ ...d, platform_keys: v }))} label={t("platformKeys")} description={t("platformKeysHint")} />
        <Field label={t("budget")} hint={t("budgetHint")} error={error === "budget" ? t("errors.budget") : undefined}>
          {({ id, describedBy, invalid }) => (
            <Input id={id} dir="ltr" inputMode="decimal" aria-describedby={describedBy} aria-invalid={invalid} value={draft.budget} onChange={(e) => setDraft((d) => ({ ...d, budget: e.target.value }))} disabled={!draft.platform_keys} />
          )}
        </Field>
        <Field label={t("markup")} hint={t("markupHint")} error={error === "markup" ? t("errors.markup") : undefined}>
          {({ id, describedBy, invalid }) => (
            <Input id={id} dir="ltr" inputMode="decimal" aria-describedby={describedBy} aria-invalid={invalid} value={draft.markup} onChange={(e) => setDraft((d) => ({ ...d, markup: e.target.value }))} disabled={!draft.platform_keys} />
          )}
        </Field>
        <fieldset disabled={!draft.platform_keys}>
          <legend className="mb-1.5 text-[13px] font-medium text-ink">{t("allowed")}</legend>
          <p className="mb-2 text-xs text-slate">{t("allowedHint")}</p>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            {ALL_PROVIDERS.map((k) => (
              <Checkbox
                key={k}
                label={ta(`providers.${k}`)}
                checked={draft.allowed.includes(k)}
                onChange={(e) => setDraft((d) => ({ ...d, allowed: e.target.checked ? [...d.allowed, k] : d.allowed.filter((x) => x !== k) }))}
              />
            ))}
          </div>
        </fieldset>
        {save.error ? <ErrorNotice error={save.error} /> : null}
      </div>
    </Dialog>
  );
}
