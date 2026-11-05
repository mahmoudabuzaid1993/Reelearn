"use client";

import { useTranslations } from "next-intl";
import { useState } from "react";
import { useUpdateTenant } from "@/lib/api/hooks/tenant";
import type { Tenant, TenantSettings } from "@/lib/api/types";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Input, Switch } from "@/components/ui/input";
import { useToast } from "@/components/ui/toast";

const DAYS = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"] as const;
type Window = NonNullable<TenantSettings["delivery_windows"]>[number];

export function DeliverySettingsForm({ tenant, onSaved, submitLabel }: { tenant: Tenant; onSaved?: () => void; submitLabel?: string }) {
  const t = useTranslations("settings.delivery");
  const update = useUpdateTenant();
  const { toast } = useToast();
  const [settings, setSettings] = useState<TenantSettings>({
    reels_per_day: 2,
    max_reviews_per_day: 1,
    prayer_aware: true,
    ramadan_aware: true,
    weekend_days: ["fri", "sat"],
    delivery_windows: [{ days: ["sun", "mon", "tue", "wed", "thu"], start: "08:30", end: "11:30" }],
    retention_months: 36,
    streak_freezes_per_week: 1,
    ...(tenant.settings as Partial<TenantSettings> | undefined),
  });
  const windows = settings.delivery_windows ?? [];

  const setWindow = (i: number, patch: Partial<Window>) =>
    setSettings((s) => ({ ...s, delivery_windows: (s.delivery_windows ?? []).map((w, j) => (j === i ? { ...w, ...patch } : w)) }));

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        update.mutate(
          { settings },
          {
            onSuccess: () => {
              toast({ title: t("saved"), tone: "success" });
              onSaved?.();
            },
          },
        );
      }}
      className="flex flex-col gap-4"
    >
      <Card>
        <CardHeader title={t("cadence")} description={t("cadenceHint")} />
        <CardBody className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Field label={t("reelsPerDay")} hint={t("reelsPerDayHint")}>
            {({ id, describedBy }) => (
              <Input id={id} type="number" min={1} max={5} aria-describedby={describedBy} value={settings.reels_per_day ?? 2} onChange={(e) => setSettings((s) => ({ ...s, reels_per_day: Number(e.target.value) }))} />
            )}
          </Field>
          <Field label={t("reviewsPerDay")} hint={t("reviewsPerDayHint")}>
            {({ id, describedBy }) => (
              <Input id={id} type="number" min={0} max={3} aria-describedby={describedBy} value={settings.max_reviews_per_day ?? 1} onChange={(e) => setSettings((s) => ({ ...s, max_reviews_per_day: Number(e.target.value) }))} />
            )}
          </Field>
        </CardBody>
      </Card>

      <Card>
        <CardHeader
          title={t("windows")}
          description={t("windowsHint")}
          actions={
            <Button size="sm" onClick={() => setSettings((s) => ({ ...s, delivery_windows: [...(s.delivery_windows ?? []), { days: ["sun", "mon", "tue", "wed", "thu"], start: "16:00", end: "19:00" }] }))}>
              {t("addWindow")}
            </Button>
          }
        />
        <CardBody className="flex flex-col gap-4">
          {windows.map((w, i) => (
            <fieldset key={i} className="rounded-[10px] border border-line p-3">
              <legend className="px-1 text-xs font-medium text-slate">{t("window", { n: i + 1 })}</legend>
              <div className="flex flex-wrap items-end gap-3">
                <div role="group" aria-label={t("days")} className="flex flex-wrap gap-1">
                  {DAYS.map((d) => {
                    const on = w.days?.includes(d);
                    return (
                      <button
                        key={d}
                        type="button"
                        aria-pressed={on}
                        onClick={() => setWindow(i, { days: on ? (w.days ?? []).filter((x) => x !== d) : [...(w.days ?? []), d] })}
                        className={`h-8 rounded-md border px-2 text-xs font-medium ${on ? "border-ink bg-ink text-white" : "border-line bg-surface text-slate"}`}
                      >
                        {t(`dayNames.${d}`)}
                      </button>
                    );
                  })}
                </div>
                <Field label={t("start")}>
                  {({ id }) => <Input id={id} type="time" dir="ltr" className="w-32" value={w.start ?? ""} onChange={(e) => setWindow(i, { start: e.target.value })} />}
                </Field>
                <Field label={t("end")}>
                  {({ id }) => <Input id={id} type="time" dir="ltr" className="w-32" value={w.end ?? ""} onChange={(e) => setWindow(i, { end: e.target.value })} />}
                </Field>
                <Button size="sm" variant="quiet" onClick={() => setSettings((s) => ({ ...s, delivery_windows: (s.delivery_windows ?? []).filter((_, j) => j !== i) }))}>
                  {t("remove")}
                </Button>
              </div>
            </fieldset>
          ))}
          {windows.length === 0 ? <p className="text-sm text-slate">{t("noWindows")}</p> : null}
        </CardBody>
      </Card>

      <Card>
        <CardHeader title={t("calendar")} />
        <CardBody className="flex flex-col gap-4">
          <Switch checked={settings.prayer_aware ?? true} onChange={(v) => setSettings((s) => ({ ...s, prayer_aware: v }))} label={t("prayerAware")} description={t("prayerAwareHint")} />
          <Switch checked={settings.ramadan_aware ?? true} onChange={(v) => setSettings((s) => ({ ...s, ramadan_aware: v }))} label={t("ramadanAware")} description={t("ramadanAwareHint")} />
          <div role="group" aria-label={t("weekend")} className="flex flex-wrap items-center gap-2">
            <span className="text-sm text-ink">{t("weekend")}</span>
            {DAYS.map((d) => {
              const on = settings.weekend_days?.includes(d);
              return (
                <button
                  key={d}
                  type="button"
                  aria-pressed={on}
                  onClick={() => setSettings((s) => ({ ...s, weekend_days: on ? (s.weekend_days ?? []).filter((x) => x !== d) : [...(s.weekend_days ?? []), d] }))}
                  className={`h-8 rounded-md border px-2 text-xs font-medium ${on ? "border-ink bg-ink text-white" : "border-line bg-surface text-slate"}`}
                >
                  {t(`dayNames.${d}`)}
                </button>
              );
            })}
          </div>
        </CardBody>
      </Card>

      {update.error ? <ErrorNotice error={update.error} /> : null}
      <div className="flex justify-end">
        <Button type="submit" variant="primary" loading={update.isPending}>
          {submitLabel ?? t("save")}
        </Button>
      </div>
    </form>
  );
}
