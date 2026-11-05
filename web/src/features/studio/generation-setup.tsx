"use client";

import Link from "next/link";
import { useLocale, useTranslations } from "next-intl";
import { useRouter, useSearchParams } from "next/navigation";
import { useMemo, useState } from "react";
import { usePeople } from "@/lib/api/hooks/people";
import { useQuote, useSource, useStartJob } from "@/lib/api/hooks/studio";
import { useTopics } from "@/lib/api/hooks/topics";
import { isApiError } from "@/lib/api/errors";
import type { GenerationRequest } from "@/lib/api/types";
import { formatNumber } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Checkbox, Input, Select, Textarea } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton, SkeletonRows } from "@/components/ui/skeleton";
import { creditsAfter, isAffordable, shortfall } from "./logic/quote";
import { generationSchema } from "./studio.schema";

const LANGS = ["ar", "en", "ur", "bn", "hi", "tl"] as const;
const STYLES = ["ai_avatar", "motion", "broll"] as const;
const VOICES = [
  { id: "layla-ar-sa", name: "Layla (ar-SA)" },
  { id: "faris-ar-sa", name: "Faris (ar-SA)" },
  { id: "maya-en-gb", name: "Maya (en-GB)" },
];

export function GenerationSetup() {
  const t = useTranslations("studio.setup");
  const tc = useTranslations("common");
  const tv = useTranslations("validation");
  const locale = useLocale();
  const router = useRouter();
  const params = useSearchParams();
  const sourceId = params.get("source") ?? "";
  const source = useSource(sourceId);
  const topics = useTopics();
  const smes = usePeople({ role: "sme" });
  const start = useStartJob();

  const [topicId, setTopicId] = useState("");
  const [languages, setLanguages] = useState<string[]>(["ar", "en"]);
  const [reelCount, setReelCount] = useState(3);
  const [style, setStyle] = useState<(typeof STYLES)[number]>("ai_avatar");
  const [voice, setVoice] = useState(VOICES[0]!.id);
  const [reviewer, setReviewer] = useState("");
  const [brief, setBrief] = useState("");
  const [confirm, setConfirm] = useState(false);

  const effectiveTopic = topicId || topics.data?.[0]?.id || "";
  const request = useMemo<GenerationRequest | null>(
    () =>
      sourceId && effectiveTopic
        ? { source_id: sourceId, topic_id: effectiveTopic, languages, reel_count: reelCount, style, presenter: { voice_id: voice }, reviewer_id: reviewer || undefined, brief: brief || undefined }
        : null,
    [sourceId, effectiveTopic, languages, reelCount, style, voice, reviewer, brief],
  );
  const quote = useQuote(request);
  const validation = request ? generationSchema.safeParse(request) : null;
  const affordable = quote.data ? isAffordable(quote.data) : false;
  const insufficient = isApiError(start.error) && start.error.status === 402;

  if (!sourceId) return <EmptyState title={t("noSource")} action={<Link href="/studio/new" className="underline">{t("pickSource")}</Link>} />;

  return (
    <>
      <PageHeader eyebrow={t("step")} title={t("title")} description={t("subtitle")} />
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[1fr_20rem]">
        <div className="flex flex-col gap-4">
          <Card>
            <CardHeader title={t("source")} />
            <CardBody>
              {source.isPending ? <SkeletonRows rows={3} /> : source.isError ? <ErrorNotice error={source.error} /> : (
                <div className="flex flex-col gap-2 text-sm">
                  <p className="font-medium text-ink">{source.data.title}</p>
                  <ul className="list-inside list-disc text-slate">
                    {((source.data.summary as { key_ideas?: string[] } | null)?.key_ideas ?? []).map((k) => (
                      <li key={k}>{k}</li>
                    ))}
                  </ul>
                </div>
              )}
            </CardBody>
          </Card>

          <Card>
            <CardHeader title={t("settings")} />
            <CardBody className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label={t("topic")} required error={validation && !validation.success && validation.error.issues.some((i) => i.path[0] === "topic_id") ? tv("required") : null}>
                {({ id }) => (
                  <Select id={id} value={effectiveTopic} onChange={(e) => setTopicId(e.target.value)}>
                    {(topics.data ?? []).map((tp) => (
                      <option key={tp.id} value={tp.id}>
                        {tp.name}
                      </option>
                    ))}
                  </Select>
                )}
              </Field>
              <Field label={t("reelCount")} hint={t("reelCountHint")}>
                {({ id, describedBy }) => <Input id={id} type="number" min={1} max={12} aria-describedby={describedBy} value={reelCount} onChange={(e) => setReelCount(Math.max(1, Math.min(12, Number(e.target.value) || 1)))} />}
              </Field>
              <fieldset className="sm:col-span-2">
                <legend className="mb-1.5 text-[13px] font-medium text-ink">{t("languages")}</legend>
                <div className="flex flex-wrap gap-3">
                  {LANGS.map((l) => (
                    <Checkbox
                      key={l}
                      label={tc(`languages.${l}`)}
                      checked={languages.includes(l)}
                      onChange={(e) => setLanguages((prev) => (e.target.checked ? [...prev, l] : prev.filter((x) => x !== l)))}
                    />
                  ))}
                </div>
                {languages.length === 0 ? <p role="alert" className="mt-1 text-xs font-medium text-ink">{tv("languages")}</p> : null}
                <p className="mt-1 text-xs text-slate">{t("languagesHint")}</p>
              </fieldset>
              <fieldset className="sm:col-span-2">
                <legend className="mb-1.5 text-[13px] font-medium text-ink">{t("style")}</legend>
                <div className="grid grid-cols-1 gap-2 sm:grid-cols-3">
                  {STYLES.map((s) => (
                    <label key={s} className={`flex cursor-pointer flex-col gap-1 rounded-[10px] border p-3 ${style === s ? "border-ink bg-ink-soft" : "border-line"}`}>
                      <span className="flex items-center gap-2">
                        <input type="radio" name="style" value={s} checked={style === s} onChange={() => setStyle(s)} className="accent-ink" />
                        <span className="font-medium text-ink">{t(`styles.${s}`)}</span>
                      </span>
                      <span className="text-xs text-slate">{t(`styleHints.${s}`)}</span>
                    </label>
                  ))}
                </div>
              </fieldset>
              <Field label={t("voice")}>
                {({ id }) => (
                  <Select id={id} value={voice} onChange={(e) => setVoice(e.target.value)}>
                    {VOICES.map((v) => (
                      <option key={v.id} value={v.id}>
                        {v.name}
                      </option>
                    ))}
                  </Select>
                )}
              </Field>
              <Field label={t("reviewer")} hint={t("reviewerHint")}>
                {({ id, describedBy }) => (
                  <Select id={id} aria-describedby={describedBy} value={reviewer} onChange={(e) => setReviewer(e.target.value)}>
                    <option value="">{t("noReviewer")}</option>
                    {(smes.data?.items ?? []).map((u) => (
                      <option key={u.id} value={u.id}>
                        {u.full_name}
                      </option>
                    ))}
                  </Select>
                )}
              </Field>
              <Field label={t("brief")} hint={t("briefHint")} className="sm:col-span-2">
                {({ id, describedBy }) => <Textarea id={id} aria-describedby={describedBy} value={brief} onChange={(e) => setBrief(e.target.value)} maxLength={600} />}
              </Field>
            </CardBody>
          </Card>
        </div>

        <aside className="lg:sticky lg:top-6 lg:self-start">
          <Card>
            <CardHeader title={t("quote")} description={t("quoteHint")} />
            <CardBody className="flex flex-col gap-3 text-sm" aria-live="polite">
              {quote.isPending || quote.isFetching ? (
                <>
                  <Skeleton /> <Skeleton className="w-2/3" /> <Skeleton className="w-1/2" />
                </>
              ) : quote.isError ? (
                <ErrorNotice error={quote.error} onRetry={() => void quote.refetch()} />
              ) : quote.data ? (
                <>
                  <ul className="flex flex-col gap-1">
                    {(quote.data.breakdown ?? []).map((line) => (
                      <li key={line.step} className="flex justify-between text-slate">
                        <span>{t(`steps.${line.step}`)}</span>
                        <span className="tabular-nums">{formatNumber(line.credits, locale)}</span>
                      </li>
                    ))}
                  </ul>
                  <div className="flex justify-between border-t border-line pt-2 font-medium text-ink">
                    <span>{t("total")}</span>
                    <span className="tabular-nums" data-testid="quote-total">
                      {t("creditsN", { n: formatNumber(quote.data.cost_credits, locale) })}
                    </span>
                  </div>
                  <div className="flex justify-between text-slate">
                    <span>{t("available")}</span>
                    <span className="tabular-nums">{formatNumber(quote.data.credits_available, locale)}</span>
                  </div>
                  <div className="flex justify-between text-slate">
                    <span>{t("after")}</span>
                    <span className="tabular-nums">{formatNumber(creditsAfter(quote.data), locale)}</span>
                  </div>
                  <p className="text-xs text-slate">{t("eta", { minutes: quote.data.estimated_minutes ?? 0 })}</p>
                  {!affordable ? (
                    <div role="alert" className="rounded-[10px] border border-ink/30 px-3 py-2 text-xs text-ink">
                      {t("insufficient", { n: formatNumber(shortfall(quote.data), locale) })}{" "}
                      <Link href="/settings#credits" className="underline underline-offset-2">
                        {t("viewPlan")}
                      </Link>
                    </div>
                  ) : null}
                </>
              ) : null}
              <Checkbox label={t("confirm")} checked={confirm} onChange={(e) => setConfirm(e.target.checked)} disabled={!quote.data || !affordable} />
              {start.error ? (
                insufficient ? (
                  <div role="alert" className="rounded-[10px] border border-ink/30 px-3 py-2 text-xs text-ink">
                    {start.error.message}{" "}
                    <Link href="/settings#credits" className="underline underline-offset-2">
                      {t("viewPlan")}
                    </Link>
                  </div>
                ) : (
                  <ErrorNotice error={start.error} />
                )
              ) : null}
              <Button
                variant="primary"
                disabled={!confirm || !affordable || !validation?.success}
                loading={start.isPending}
                onClick={() => request && start.mutate(request, { onSuccess: (job) => router.push(`/studio/jobs/${job.id}`) })}
              >
                {t("start")}
              </Button>
              <Badge tone="outline" className="self-start">
                {t("reviewable")}
              </Badge>
            </CardBody>
          </Card>
        </aside>
      </div>
    </>
  );
}
