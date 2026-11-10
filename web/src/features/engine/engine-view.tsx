"use client";

import { useLocale, useTranslations } from "next-intl";
import { useEffect, useMemo, useState } from "react";
import { useEngine, useEngineInsights, useSaveEngine } from "@/lib/api/hooks/engine";
import type { Range } from "@/lib/api/hooks/reports";
import type { EngineInsights, EngineSettings, Intensity } from "@/lib/api/types";
import { formatDate, formatNumber, formatPct } from "@/lib/format";
import { cn } from "@/lib/cn";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Switch } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton, SkeletonRows } from "@/components/ui/skeleton";
import { Stat } from "@/components/ui/stat";
import { Table, Td, Th, Tr } from "@/components/ui/table";
import { Tabs } from "@/components/ui/tabs";
import { useToast } from "@/components/ui/toast";
import {
  changedFields,
  GROUPS,
  labelFor,
  NEGATIVE_WEIGHTS,
  NUMBER_RANGES,
  patchFor,
  POSITIVE_WEIGHTS,
  PRESETS,
  type NumberKey,
  type Range as NumRange,
  setNumber,
  setToggle,
  setWeight,
  WEIGHT_RANGES,
  type WeightKey,
} from "./logic/engine";

type Tab = "tuning" | "insights";

export function EngineView() {
  const t = useTranslations("engine");
  const [tab, setTab] = useState<Tab>("tuning");
  return (
    <>
      <PageHeader title={t("title")} description={t("subtitle")} />
      <Tabs ariaLabel={t("title")} value={tab} onChange={setTab} items={[{ value: "tuning", label: t("tabs.tuning") }, { value: "insights", label: t("tabs.insights") }]} />
      <div className="mt-6">{tab === "tuning" ? <Tuning /> : <Insights />}</div>
    </>
  );
}

// ------------------------------------------------------------------ tuning

function Tuning() {
  const t = useTranslations("engine");
  const engine = useEngine();
  const save = useSaveEngine();
  const { toast } = useToast();
  const [draft, setDraft] = useState<EngineSettings | null>(null);
  const saved = engine.data?.settings ?? null;
  const presets = useMemo(() => (engine.data?.presets ?? {}) as Record<string, EngineSettings>, [engine.data]);

  useEffect(() => {
    if (saved) setDraft(saved);
  }, [saved]);

  if (engine.isPending || !draft || !saved) return <SkeletonRows rows={6} />;
  if (engine.isError) return <ErrorNotice error={engine.error} onRetry={() => void engine.refetch()} />;

  const changes = changedFields(saved, draft);
  const label: Intensity = labelFor(draft, presets);

  const applyPreset = (p: Exclude<Intensity, "custom">) =>
    save.mutate(
      { intensity: p },
      {
        onSuccess: () => toast({ tone: "success", title: t("savedPreset", { preset: t(`presets.${p}.name`) }) }),
        onError: () => toast({ tone: "neutral", title: t("saveFailed") }),
      },
    );
  const saveDraft = () =>
    save.mutate(patchFor(saved, draft), {
      onSuccess: () => toast({ tone: "success", title: t("saved") }),
      onError: () => toast({ tone: "neutral", title: t("saveFailed") }),
    });

  return (
    <div className="flex flex-col gap-6 pb-20">
      <Card>
        <CardHeader
          title={t("intensity")}
          description={t("intensityHint")}
          actions={label === "custom" ? <Badge tone="mint">{t("presets.custom.name")}</Badge> : null}
        />
        <CardBody>
          <div role="radiogroup" aria-label={t("intensity")} className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            {PRESETS.map((p) => {
              const preset = presets[p];
              const active = saved.intensity === p && changes.length === 0;
              return (
                <button
                  key={p}
                  type="button"
                  role="radio"
                  aria-checked={active}
                  disabled={save.isPending}
                  onClick={() => applyPreset(p)}
                  className={cn(
                    "flex flex-col items-start gap-2 rounded-[12px] border p-4 text-start transition-colors",
                    active ? "border-ink bg-ink text-white" : "border-line bg-surface hover:border-ink",
                  )}
                >
                  <span className="font-display text-[15px] font-semibold">{t(`presets.${p}.name`)}</span>
                  <span className={cn("text-[13px]", active ? "text-white/80" : "text-slate")}>{t(`presets.${p}.hint`)}</span>
                  {preset ? (
                    <ul className={cn("mt-1 space-y-0.5 text-xs", active ? "text-white/80" : "text-slate")}>
                      <li>
                        {!preset.continuous_enabled
                          ? t("facts.noPractice")
                          : preset.daily_minutes_cap === 0
                            ? t("facts.practiceNoCap")
                            : t("facts.practiceCap", { minutes: preset.daily_minutes_cap })}
                      </li>
                      <li>{t("facts.reminders", { count: preset.max_nudges_per_day })}</li>
                      <li>{preset.break_reminder_minutes === 0 ? t("facts.noBreaks") : t("facts.breaks", { minutes: preset.break_reminder_minutes })}</li>
                    </ul>
                  ) : null}
                </button>
              );
            })}
          </div>
        </CardBody>
      </Card>

      <Card>
        <CardHeader title={t("valueModel")} description={t("valueModelHint")} />
        <CardBody className="grid gap-8 lg:grid-cols-2">
          <div className="flex flex-col gap-5">
            <h3 className="text-xs font-medium uppercase tracking-wide text-slate">{t("raises")}</h3>
            {POSITIVE_WEIGHTS.map((w) => (
              <WeightSlider key={w} name={w} value={draft.weights[w] ?? 0} onChange={(v) => setDraft(setWeight(draft, w, v))} />
            ))}
          </div>
          <div className="flex flex-col gap-5">
            <h3 className="text-xs font-medium uppercase tracking-wide text-slate">{t("lowers")}</h3>
            {NEGATIVE_WEIGHTS.map((w) => (
              <WeightSlider key={w} name={w} value={draft.weights[w] ?? 0} onChange={(v) => setDraft(setWeight(draft, w, v))} />
            ))}
            <p className="rounded-[10px] bg-surface-muted p-3 text-xs text-slate">{t("guardrails")}</p>
          </div>
        </CardBody>
      </Card>

      <div className="grid gap-6 lg:grid-cols-2">
        {GROUPS.map((g) => (
          <Card key={g.key}>
            <CardHeader title={t(`groups.${g.key}.title`)} description={t(`groups.${g.key}.hint`)} />
            <CardBody className="flex flex-col gap-5">
              {g.toggles.map((k) => (
                <Switch key={k} checked={draft[k]} onChange={(v) => setDraft(setToggle(draft, k, v))} label={t(`fields.${k}.label`)} description={t(`fields.${k}.hint`)} />
              ))}
              {g.numbers.map((k) => (
                <NumberSlider key={k} name={k} value={draft[k]} onChange={(v) => setDraft(setNumber(draft, k, v))} disabled={isPracticeField(k) && !draft.continuous_enabled} />
              ))}
            </CardBody>
          </Card>
        ))}
      </div>

      {changes.length > 0 ? (
        <div role="region" aria-label={t("unsaved", { count: changes.length })} className="fixed inset-x-0 bottom-0 z-20 border-t border-line bg-surface/95 px-6 py-3 backdrop-blur md:ps-64">
          <div className="mx-auto flex max-w-6xl items-center justify-between gap-4">
            <p className="text-sm text-ink">{t("unsaved", { count: changes.length })}</p>
            <div className="flex gap-2">
              <Button variant="quiet" onClick={() => setDraft(saved)} disabled={save.isPending}>
                {t("discard")}
              </Button>
              <Button variant="primary" onClick={saveDraft} loading={save.isPending}>
                {t("save")}
              </Button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function isPracticeField(k: NumberKey): boolean {
  return k === "daily_minutes_cap" || k === "session_minutes_cap" || k === "break_reminder_minutes" || k === "batch_size";
}

function display(value: number, r: NumRange, t: ReturnType<typeof useTranslations>, locale: string, unit?: string): string {
  if (value === 0 && r.zeroLabel) return t(`zero.${r.zeroLabel}`);
  if (r.percent) return formatPct(value * 100, locale);
  const n = formatNumber(value, locale, r.step < 1 ? 1 : 0);
  return unit ? t(`units.${unit}`, { value: n }) : n;
}

function Slider({ id, label, hint, value, range, valueText, onChange, disabled }: { id: string; label: string; hint: string; value: number; range: NumRange; valueText: string; onChange: (v: number) => void; disabled?: boolean }) {
  return (
    <div className={cn("flex flex-col gap-1.5", disabled && "opacity-50")}>
      <div className="flex items-baseline justify-between gap-3">
        <label htmlFor={id} className="text-[13px] font-medium text-ink">
          {label}
        </label>
        <output htmlFor={id} className="font-display text-sm font-semibold tabular-nums text-ink">
          {valueText}
        </output>
      </div>
      <input
        id={id}
        type="range"
        min={range.min}
        max={range.max}
        step={range.step}
        value={value}
        disabled={disabled}
        aria-valuetext={valueText}
        aria-describedby={`${id}-hint`}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full accent-ink"
      />
      <p id={`${id}-hint`} className="text-xs text-slate">
        {hint}
      </p>
    </div>
  );
}

function WeightSlider({ name, value, onChange }: { name: WeightKey; value: number; onChange: (v: number) => void }) {
  const t = useTranslations("engine");
  const locale = useLocale();
  return (
    <Slider
      id={`w-${name}`}
      label={t(`weights.${name}.label`)}
      hint={t(`weights.${name}.hint`)}
      value={value}
      range={WEIGHT_RANGES[name]}
      valueText={value === 0 ? t("zero.off") : formatNumber(value, locale, 1)}
      onChange={onChange}
    />
  );
}

const UNITS: Partial<Record<NumberKey, string>> = {
  daily_minutes_cap: "minutes",
  session_minutes_cap: "minutes",
  break_reminder_minutes: "minutes",
  interest_half_life_days: "days",
  reengage_after_hours: "hours",
  max_nudges_per_day: "perDay",
  batch_size: "items",
  review_every: "everyN",
  max_same_topic_in_row: "inARow",
};

function NumberSlider({ name, value, onChange, disabled }: { name: NumberKey; value: number; onChange: (v: number) => void; disabled?: boolean }) {
  const t = useTranslations("engine");
  const locale = useLocale();
  const r = NUMBER_RANGES[name];
  return <Slider id={`n-${name}`} label={t(`fields.${name}.label`)} hint={t(`fields.${name}.hint`)} value={value} range={r} valueText={display(value, r, t, locale, UNITS[name])} onChange={onChange} disabled={disabled} />;
}

// ------------------------------------------------------------------ insights

function Insights() {
  const t = useTranslations("engine.insights");
  const te = useTranslations("engine");
  const tr = useTranslations("reports");
  const locale = useLocale();
  const [range, setRange] = useState<Range>("30d");
  const q = useEngineInsights(range);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-sm text-slate">{q.data ? t("intensityNow", { intensity: te(`presets.${q.data.intensity}.name`) }) : null}</p>
        <div role="group" aria-label={tr("range")} className="flex rounded-[10px] border border-line bg-surface p-0.5">
          {(["7d", "30d", "90d", "12m"] as Range[]).map((r) => (
            <button key={r} type="button" aria-pressed={range === r} onClick={() => setRange(r)} className={`h-8 rounded-lg px-3 text-[13px] font-medium ${range === r ? "bg-ink text-white" : "text-slate hover:text-ink"}`}>
              {tr(`ranges.${r}`)}
            </button>
          ))}
        </div>
      </div>
      {q.isError ? <ErrorNotice error={q.error} onRetry={() => void q.refetch()} /> : null}
      {q.isPending ? (
        <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
          {Array.from({ length: 8 }, (_, i) => (
            <Card key={i} className="px-5 py-4">
              <Skeleton className="h-3 w-20" />
              <Skeleton className="mt-3 h-7 w-16" />
            </Card>
          ))}
        </div>
      ) : q.data ? (
        <InsightsBody data={q.data} locale={locale} />
      ) : null}
      <p className="text-xs text-slate">{t("privacy")}</p>
    </div>
  );
}

function Bar({ value, max, tone = "ink" }: { value: number; max: number; tone?: "ink" | "mint" | "coral" }) {
  const pct = max > 0 ? Math.max(2, Math.round((value / max) * 100)) : 0;
  return (
    <div className="h-2 w-full rounded-full bg-surface-muted" aria-hidden>
      <div className={cn("h-2 rounded-full", tone === "mint" ? "bg-mint" : tone === "coral" ? "bg-coral" : "bg-ink")} style={{ width: `${pct}%` }} />
    </div>
  );
}

function InsightsBody({ data, locale }: { data: EngineInsights; locale: string }) {
  const t = useTranslations("engine.insights");
  const maxMinutes = Math.max(0, ...data.daily.map((d) => d.practice_minutes));
  const maxReason = Math.max(0, ...data.reasons.map((r) => r.impressions));
  const maxTag = Math.max(0, ...data.tags.map((x) => x.interested));
  const pct = (v: number) => formatPct(v * 100, locale);
  return (
    <>
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <Stat label={t("practiceMinutes")} value={formatNumber(data.practice_minutes, locale)} hint={t("learners", { count: data.learners_practising })} />
        <Stat label={t("avgSession")} value={t("minutesShort", { value: formatNumber(data.avg_session_minutes, locale, 1) })} hint={t("itemsPerSession", { value: formatNumber(data.items_per_session, locale, 1) })} />
        <Stat label={t("perLearner")} value={t("minutesShort", { value: formatNumber(data.avg_minutes_per_learner, locale, 1) })} hint={t("sessions", { count: data.sessions })} />
        <Stat label={t("completion")} value={pct(data.completion_rate)} hint={t("skipRate", { value: pct(data.skip_rate) })} />
        <Stat label={t("shares")} value={formatNumber(data.sharing.shares, locale)} hint={t("opened", { value: pct(data.sharing.opened_rate) })} />
        <Stat label={t("sharers")} value={formatNumber(data.sharing.senders, locale)} hint={t("recipients", { count: data.sharing.recipients })} />
        <Stat label={t("saves")} value={formatNumber(data.saves, locale)} />
        <Stat label={t("notForMe")} value={formatNumber(data.not_relevant, locale)} hint={t("notForMeHint")} />
      </div>

      <Card>
        <CardHeader title={t("daily")} description={t("dailyHint")} />
        <CardBody>
          {data.daily.length === 0 ? (
            <EmptyState title={t("noData")} />
          ) : (
            <>
              <div className="flex h-40 items-end gap-1" aria-hidden>
                {data.daily.map((d) => (
                  <div key={d.day} className="flex flex-1 flex-col items-center justify-end gap-1" title={`${formatDate(d.day, locale)} · ${d.practice_minutes}`}>
                    <div className="w-full rounded-t-[4px] bg-ink" style={{ height: `${maxMinutes > 0 ? Math.max(2, (d.practice_minutes / maxMinutes) * 100) : 0}%` }} />
                  </div>
                ))}
              </div>
              <div className="mt-2 flex justify-between text-xs text-slate" aria-hidden>
                <span>{formatDate(data.daily[0]!.day, locale)}</span>
                <span>{formatDate(data.daily[data.daily.length - 1]!.day, locale)}</span>
              </div>
              <table className="sr-only">
                <caption>{t("daily")}</caption>
                <thead>
                  <tr>
                    <th>{t("day")}</th>
                    <th>{t("practiceMinutes")}</th>
                    <th>{t("sessionsCol")}</th>
                  </tr>
                </thead>
                <tbody>
                  {data.daily.map((d) => (
                    <tr key={d.day}>
                      <td>{formatDate(d.day, locale)}</td>
                      <td>{d.practice_minutes}</td>
                      <td>{d.sessions}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}
        </CardBody>
      </Card>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader title={t("interestByTopic")} description={t("interestByTopicHint")} />
          <Table>
            <thead>
              <tr>
                <Th>{t("topic")}</Th>
                <Th className="text-end">{t("interested")}</Th>
                <Th className="text-end">{t("followers")}</Th>
                <Th className="text-end">{t("muted")}</Th>
              </tr>
            </thead>
            <tbody>
              {data.topics.map((r) => (
                <Tr key={r.topic_id}>
                  <Td>{r.name}</Td>
                  <Td className="text-end tabular-nums">{formatNumber(r.interested, locale)}</Td>
                  <Td className="text-end tabular-nums">{formatNumber(r.followers, locale)}</Td>
                  <Td className="text-end tabular-nums">{formatNumber(r.muted, locale)}</Td>
                </Tr>
              ))}
            </tbody>
          </Table>
        </Card>
        <Card>
          <CardHeader title={t("interestByTag")} description={t("interestByTagHint")} />
          <CardBody className="flex flex-col gap-3">
            {data.tags.length === 0 ? <p className="text-sm text-slate">{t("noData")}</p> : null}
            {data.tags.map((x) => (
              <div key={x.tag} className="grid grid-cols-[8rem_1fr_3rem] items-center gap-3 text-sm">
                <span className="truncate text-ink">{x.tag}</span>
                <Bar value={x.interested} max={maxTag} tone="mint" />
                <span className="text-end tabular-nums text-slate">{formatNumber(x.interested, locale)}</span>
              </div>
            ))}
          </CardBody>
        </Card>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader title={t("whyShown")} description={t("whyShownHint")} />
          <CardBody className="flex flex-col gap-3">
            {data.reasons.map((r) => (
              <div key={r.reason} className="grid grid-cols-[8rem_1fr_4rem] items-center gap-3 text-sm">
                <span className="text-ink">{t(`reasons.${r.reason}`)}</span>
                <Bar value={r.impressions} max={maxReason} />
                <span className="text-end tabular-nums text-slate">{formatNumber(r.impressions, locale)}</span>
              </div>
            ))}
          </CardBody>
        </Card>
        <Card>
          <CardHeader title={t("byLength")} description={t("byLengthHint")} />
          <Table>
            <thead>
              <tr>
                <Th>{t("length")}</Th>
                <Th className="text-end">{t("starts")}</Th>
                <Th className="text-end">{t("finished")}</Th>
                <Th className="text-end">{t("skipped")}</Th>
              </tr>
            </thead>
            <tbody>
              {data.bands.map((b) => (
                <Tr key={b.band}>
                  <Td>{t(`bands.${b.band}`)}</Td>
                  <Td className="text-end tabular-nums">{formatNumber(b.starts, locale)}</Td>
                  <Td className="text-end tabular-nums">{b.starts > 0 ? pct(b.completion_rate) : "–"}</Td>
                  <Td className="text-end tabular-nums">{b.starts > 0 ? pct(b.skip_rate) : "–"}</Td>
                </Tr>
              ))}
            </tbody>
          </Table>
        </Card>
      </div>

      <Card>
        <CardHeader title={t("reels")} description={t("reelsHint")} />
        <Table>
          <thead>
            <tr>
              <Th>{t("reel")}</Th>
              <Th className="text-end">{t("starts")}</Th>
              <Th className="text-end">{t("finished")}</Th>
              <Th className="text-end">{t("skipped")}</Th>
              <Th className="text-end">{t("savesCol")}</Th>
              <Th className="text-end">{t("sharesCol")}</Th>
              <Th className="text-end">{t("notForMe")}</Th>
              <Th className="text-end">{t("correct")}</Th>
            </tr>
          </thead>
          <tbody>
            {data.reels.map((r) => (
              <Tr key={r.reel_id}>
                <Td>
                  <span className="block text-ink">{r.title}</span>
                  <span className="text-xs text-slate">{r.topic_name}</span>
                </Td>
                <Td className="text-end tabular-nums">{formatNumber(r.starts, locale)}</Td>
                <Td className="text-end tabular-nums">{pct(r.completion_rate)}</Td>
                <Td className="text-end tabular-nums">{pct(r.skip_rate)}</Td>
                <Td className="text-end tabular-nums">{formatNumber(r.saves, locale)}</Td>
                <Td className="text-end tabular-nums">{formatNumber(r.shares, locale)}</Td>
                <Td className="text-end tabular-nums">{formatNumber(r.not_relevant, locale)}</Td>
                <Td className="text-end tabular-nums">{r.correct_rate == null ? "–" : pct(r.correct_rate)}</Td>
              </Tr>
            ))}
          </tbody>
        </Table>
      </Card>

      <Card>
        <CardHeader title={t("locations")} description={t("locationsHint")} />
        <Table>
          <thead>
            <tr>
              <Th>{t("location")}</Th>
              <Th className="text-end">{t("learnersCol")}</Th>
              <Th className="text-end">{t("sessionsCol")}</Th>
              <Th className="text-end">{t("practiceMinutes")}</Th>
            </tr>
          </thead>
          <tbody>
            {data.cohorts.map((c) => (
              <Tr key={c.location}>
                <Td>{c.location}</Td>
                <Td className="text-end tabular-nums">{formatNumber(c.learners, locale)}</Td>
                <Td className="text-end tabular-nums">{formatNumber(c.sessions, locale)}</Td>
                <Td className="text-end tabular-nums">{formatNumber(c.practice_minutes, locale)}</Td>
              </Tr>
            ))}
          </tbody>
        </Table>
      </Card>
    </>
  );
}
