"use client";

import { useLocale, useTranslations } from "next-intl";
import { useState } from "react";
import { useTenant, useUpdateTenant } from "@/lib/api/hooks/tenant";
import { useIntegrations, useSaveIntegration, useTestIntegration } from "@/lib/api/hooks/settings";
import type { Integration, IntegrationKind } from "@/lib/api/types";
import { formatDateTime, formatNumber } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Input, Select } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { Tabs } from "@/components/ui/tabs";
import { useToast } from "@/components/ui/toast";
import { AiPanel } from "@/features/ai/ai-panel";
import { configPatch, PRICE_FIELDS, PROVIDER_FIELDS, type ProviderKind } from "@/features/ai/logic/ai";
import { DeliverySettingsForm } from "./delivery-settings-form";

type Tab = "integrations" | "ai" | "delivery" | "residency" | "credits";

const GROUPS: { key: string; kinds: IntegrationKind[] }[] = [
  { key: "identity", kinds: ["sso_entra", "hris_jisr", "hris_bayzat"] },
  { key: "channels", kinds: ["whatsapp", "teams"] },
  { key: "ai", kinds: ["ai_muse", "ai_claude", "ai_typesafe", "ai_higgsfield", "ai_veo", "voice", "asr"] },
  { key: "storage", kinds: ["storage"] },
];

export function SettingsView() {
  const t = useTranslations("settings");
  const [tab, setTab] = useState<Tab>("integrations");
  const [editing, setEditing] = useState<IntegrationKind | null>(null);
  return (
    <>
      <PageHeader title={t("title")} description={t("subtitle")} />
      <Tabs
        ariaLabel={t("title")}
        value={tab}
        onChange={setTab}
        items={[
          { value: "integrations", label: t("tabs.integrations") },
          { value: "ai", label: t("tabs.ai") },
          { value: "delivery", label: t("tabs.delivery") },
          { value: "residency", label: t("tabs.residency") },
          { value: "credits", label: t("tabs.credits") },
        ]}
      />
      <div className="mt-6">
        {tab === "integrations" ? <IntegrationsPanel onEdit={setEditing} /> : null}
        {tab === "ai" ? <AiPanel onManageKey={(kind: ProviderKind) => setEditing(kind as IntegrationKind)} /> : null}
        {tab === "delivery" ? <DeliveryPanel /> : null}
        {tab === "residency" ? <ResidencyPanel /> : null}
        {tab === "credits" ? <CreditsPanel /> : null}
      </div>
      <IntegrationDialog kind={editing} onClose={() => setEditing(null)} />
    </>
  );
}

function statusTone(status: Integration["status"]) {
  return status === "connected" ? "mint" : status === "error" ? "coral" : "neutral";
}

function IntegrationsPanel({ onEdit }: { onEdit: (kind: IntegrationKind) => void }) {
  const t = useTranslations("settings");
  const locale = useLocale();
  const query = useIntegrations();
  const test = useTestIntegration();
  const { toast } = useToast();

  if (query.isPending) return <SkeletonRows rows={6} />;
  if (query.isError) return <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />;

  return (
    <div className="flex flex-col gap-6">
      {GROUPS.map((group) => (
        <Card key={group.key}>
          <CardHeader title={t(`groups.${group.key}`)} />
          <ul className="divide-y divide-line">
            {group.kinds.map((kind) => {
              const item = query.data.find((i) => i.kind === kind) ?? { kind, status: "disconnected" as const, last_sync_at: null, last_error: null, public_config: {} };
              return (
                <li key={kind} className="flex flex-wrap items-center justify-between gap-3 px-5 py-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="font-medium text-ink">{t(`kinds.${kind}`)}</p>
                      <Badge tone={statusTone(item.status)}>{t(`status.${item.status}`)}</Badge>
                    </div>
                    <p className="mt-0.5 truncate text-xs text-slate">
                      {item.last_error ?? Object.entries(item.public_config ?? {}).map(([k, v]) => `${k}: ${String(v)}`).join(" · ") ?? ""}
                      {item.last_sync_at ? ` · ${t("lastSync", { at: formatDateTime(item.last_sync_at, locale) })}` : ""}
                    </p>
                  </div>
                  <div className="flex gap-2">
                    <Button
                      size="sm"
                      loading={test.isPending && test.variables === kind}
                      onClick={() =>
                        test.mutate(kind, {
                          onSuccess: (r) => toast({ title: r.ok ? t("testOk") : t("testFailed"), description: r.message, tone: r.ok ? "success" : "neutral" }),
                        })
                      }
                    >
                      {t("test")}
                    </Button>
                    <Button size="sm" variant="secondary" onClick={() => onEdit(kind)}>
                      {item.status === "disconnected" ? t("connect") : t("configure")}
                    </Button>
                  </div>
                </li>
              );
            })}
          </ul>
        </Card>
      ))}
    </div>
  );
}

const FIELDS: Partial<Record<IntegrationKind, { name: string; secret?: boolean; optional?: boolean }[]>> = {
  hris_jisr: [{ name: "company_id" }, { name: "api_key", secret: true }],
  hris_bayzat: [{ name: "company_id" }, { name: "api_token", secret: true }],
  sso_entra: [{ name: "tenant_id" }, { name: "client_id" }, { name: "client_secret", secret: true }],
  teams: [{ name: "tenant_id" }, { name: "app_password", secret: true }],
  ai_veo: [{ name: "project_id" }, { name: "api_key", secret: true }],
  storage: [{ name: "bucket" }, { name: "region" }, { name: "secret_key", secret: true }],
  ...PROVIDER_FIELDS,
};

/** AI providers can carry the company's own vendor prices, so usage shows what its keys cost. */
const PRICED: IntegrationKind[] = ["ai_muse", "ai_claude", "ai_typesafe", "ai_higgsfield", "voice", "asr"];

function IntegrationDialog({ kind, onClose }: { kind: IntegrationKind | null; onClose: () => void }) {
  const t = useTranslations("settings");
  const save = useSaveIntegration();
  const { toast } = useToast();
  const [values, setValues] = useState<Record<string, string>>({});
  const [touched, setTouched] = useState<Set<string>>(new Set());
  if (!kind) return null;
  const fields = [
    ...(FIELDS[kind] ?? []),
    ...(PRICED.includes(kind) ? PRICE_FIELDS.filter((p) => kind !== "ai_typesafe" || p !== "price_per_call").map((name) => ({ name, optional: true, secret: false })) : []),
  ];
  const label = (name: string) => (t.has(`fields.${name}`) ? t(`fields.${name}` as never) : name.replace(/_/g, " "));
  const close = () => {
    setValues({});
    setTouched(new Set());
    onClose();
  };
  return (
    <Dialog
      open={Boolean(kind)}
      onClose={close}
      title={t("configureKind", { kind: t(`kinds.${kind}`) })}
      description={t("secretsWriteOnly")}
      footer={
        <>
          <Button onClick={close}>{t("cancel")}</Button>
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
            {t("save")}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-3">
        {fields.map((f) => (
          <Field key={f.name} label={f.optional ? `${label(f.name)} (${t("fields.optional")})` : label(f.name)}>
            {({ id }) => (
              <Input
                id={id}
                dir="ltr"
                type={f.secret ? "password" : "text"}
                inputMode={f.name.startsWith("price_") ? "decimal" : undefined}
                autoComplete="off"
                value={values[f.name] ?? ""}
                onChange={(e) => {
                  setValues((v) => ({ ...v, [f.name]: e.target.value }));
                  setTouched((s) => new Set(s).add(f.name));
                }}
              />
            )}
          </Field>
        ))}
        {save.error ? <ErrorNotice error={save.error} /> : null}
      </div>
    </Dialog>
  );
}

function DeliveryPanel() {
  const query = useTenant();
  if (query.isPending) return <SkeletonRows rows={6} />;
  if (query.isError) return <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />;
  return <DeliverySettingsForm tenant={query.data} />;
}

function ResidencyPanel() {
  const t = useTranslations("settings");
  const query = useTenant();
  const update = useUpdateTenant();
  const { toast } = useToast();
  if (query.isPending) return <SkeletonRows rows={4} />;
  if (query.isError) return <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />;
  const tenant = query.data;
  return (
    <Card>
      <CardHeader title={t("residency.title")} description={t("residency.hint")} />
      <CardBody className="flex flex-col gap-4">
        <dl className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate">{t("residency.region")}</dt>
            <dd className="mt-1 flex items-center gap-2 font-medium text-ink">
              {t(`regions.${tenant.region ?? "ksa"}`)} <Badge tone="ink">{t("residency.pinned")}</Badge>
            </dd>
          </div>
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate">{t("residency.retention")}</dt>
            <dd className="mt-1 font-medium text-ink">{t("residency.months", { n: tenant.settings?.retention_months ?? 36 })}</dd>
          </div>
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate">{t("residency.timezone")}</dt>
            <dd className="mt-1 font-medium text-ink" dir="ltr">
              {tenant.timezone}
            </dd>
          </div>
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate">{t("residency.languages")}</dt>
            <dd className="mt-1 flex flex-wrap gap-1">
              {tenant.languages.map((l) => (
                <Badge key={l} tone="outline">
                  {l}
                </Badge>
              ))}
            </dd>
          </div>
        </dl>
        <Field label={t("residency.defaultLanguage")}>
          {({ id }) => (
            <Select
              id={id}
              className="max-w-xs"
              value={tenant.default_language ?? "ar"}
              onChange={(e) => update.mutate({ default_language: e.target.value }, { onSuccess: () => toast({ title: t("saved"), tone: "success" }) })}
            >
              <option value="ar">العربية</option>
              <option value="en">English</option>
            </Select>
          )}
        </Field>
        <p className="text-xs text-slate">{t("residency.note")}</p>
      </CardBody>
    </Card>
  );
}

function CreditsPanel() {
  const t = useTranslations("settings");
  const locale = useLocale();
  const query = useTenant();
  if (query.isPending) return <SkeletonRows rows={3} />;
  if (query.isError) return <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />;
  return (
    <Card id="credits">
      <CardHeader title={t("credits.title")} description={t("credits.hint")} />
      <CardBody className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-xs uppercase tracking-wide text-slate">{t("credits.available")}</p>
          <p className="font-display text-3xl font-semibold text-ink">{formatNumber(query.data.credits, locale)}</p>
        </div>
        <div className="text-sm text-slate">
          <p>{t("credits.rates.avatar")}</p>
          <p>{t("credits.rates.motion")}</p>
          <p>{t("credits.rates.broll")}</p>
          <p>{t("credits.rates.dub")}</p>
        </div>
        <a href="mailto:sales@reelearn.example" className="inline-flex h-10 items-center rounded-[10px] border border-ink bg-ink px-4 font-display text-sm font-medium text-white hover:bg-night">
          {t("credits.topUp")}
        </a>
      </CardBody>
    </Card>
  );
}

