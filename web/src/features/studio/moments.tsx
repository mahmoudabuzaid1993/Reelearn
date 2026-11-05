"use client";

import Link from "next/link";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { useCutMoment, useMoments, useSource, useTranscript } from "@/lib/api/hooks/studio";
import { useTopics } from "@/lib/api/hooks/topics";
import type { Moment } from "@/lib/api/types";
import { formatDuration } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Checkbox, Input, Select } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { useToast } from "@/components/ui/toast";
import { cn } from "@/lib/cn";
import { cutSchema } from "./studio.schema";

export function Moments({ sourceId }: { sourceId: string }) {
  const t = useTranslations("studio.moments");
  const tv = useTranslations("validation");
  const router = useRouter();
  const { toast } = useToast();
  const source = useSource(sourceId);
  const moments = useMoments(sourceId);
  const transcript = useTranscript(sourceId);
  const topics = useTopics();
  const cut = useCutMoment();
  const [selected, setSelected] = useState<string | null>(null);
  const [form, setForm] = useState<{ title: string; start_ms: number; end_ms: number; topic_id: string; speaker_consent: boolean }>({ title: "", start_ms: 0, end_ms: 0, topic_id: "", speaker_consent: false });
  const [errors, setErrors] = useState<Record<string, string>>({});

  const pick = (m: Moment) => {
    setSelected(m.id);
    setForm({ title: m.title ?? "", start_ms: m.start_ms, end_ms: m.end_ms, topic_id: form.topic_id || topics.data?.[0]?.id || "", speaker_consent: false });
    setErrors({});
  };

  if (source.isPending || moments.isPending) return <SkeletonRows rows={8} />;
  if (source.isError) return <ErrorNotice error={source.error} onRetry={() => void source.refetch()} />;
  if (moments.isError) return <ErrorNotice error={moments.error} onRetry={() => void moments.refetch()} />;

  const current = moments.data.find((m) => m.id === selected) ?? null;

  const submit = () => {
    const parsed = cutSchema.safeParse(form);
    if (!parsed.success) {
      setErrors(Object.fromEntries(parsed.error.issues.map((i) => [String(i.path[0] ?? "form"), i.message])));
      return;
    }
    if (!current) return;
    cut.mutate(
      { sourceId, momentId: current.id, body: parsed.data },
      {
        onSuccess: (reel) => {
          toast({ title: t("cutDone"), tone: "success" });
          router.push(`/studio/reels/${reel.id}`);
        },
      },
    );
  };

  return (
    <>
      <PageHeader eyebrow={t("step")} title={t("title")} description={source.data.title} />
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[1fr_22rem]">
        <div className="flex flex-col gap-4">
          <Card>
            <CardHeader title={t("aiMoments")} description={t("aiMomentsHint")} />
            {moments.data.length === 0 ? (
              <CardBody>
                <EmptyState title={t("noMoments")} description={t("noMomentsHint")} />
              </CardBody>
            ) : (
              <ul className="divide-y divide-line">
                {moments.data.map((m) => (
                  <li key={m.id}>
                    <button
                      type="button"
                      aria-pressed={selected === m.id}
                      onClick={() => pick(m)}
                      className={cn("flex w-full flex-col gap-1 px-5 py-3 text-start hover:bg-paper", selected === m.id && "bg-ink-soft")}
                    >
                      <span className="flex flex-wrap items-center gap-2">
                        <span className="font-medium text-ink">{m.title ?? t("untitled")}</span>
                        <span className="text-xs text-slate" dir="ltr">
                          {formatDuration(m.start_ms)}–{formatDuration(m.end_ms)}
                        </span>
                        {m.speaker ? <Badge tone="outline">{m.speaker}</Badge> : null}
                        <Badge tone="neutral">{t("score", { n: Math.round(m.score * 100) })}</Badge>
                        {m.reel_id ? (
                          <Link href={`/studio/reels/${m.reel_id}`} className="text-xs text-ink underline underline-offset-2" onClick={(e) => e.stopPropagation()}>
                            {t("alreadyCut")}
                          </Link>
                        ) : null}
                      </span>
                      <span className="text-sm text-slate">{m.excerpt}</span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </Card>
          <Card>
            <CardHeader title={t("transcript")} description={t("transcriptHint")} />
            <CardBody className="max-h-96 overflow-auto">
              {transcript.isPending ? (
                <SkeletonRows rows={5} />
              ) : transcript.isError ? (
                <ErrorNotice error={transcript.error} />
              ) : (
                <ol className="flex flex-col gap-2 text-sm">
                  {transcript.data.map((line) => {
                    const inRange = current && line.end_ms > form.start_ms && line.start_ms < form.end_ms;
                    return (
                      <li key={line.start_ms} className={cn("grid grid-cols-[4.5rem_6rem_1fr] gap-2 rounded-md px-2 py-1", inRange && "bg-mint-soft")}>
                        <button type="button" className="text-start text-xs tabular-nums text-slate underline-offset-2 hover:underline" dir="ltr" onClick={() => setForm((f) => ({ ...f, start_ms: line.start_ms, end_ms: Math.max(line.end_ms, line.start_ms + 10_000) }))}>
                          {formatDuration(line.start_ms)}
                        </button>
                        <span className="text-xs font-medium text-ink">{line.speaker}</span>
                        <span className="text-ink">{line.text}</span>
                      </li>
                    );
                  })}
                </ol>
              )}
            </CardBody>
          </Card>
        </div>

        <aside className="lg:sticky lg:top-6 lg:self-start">
          <Card>
            <CardHeader title={t("cut")} description={current ? t("cutHint") : t("pickFirst")} />
            <CardBody className="flex flex-col gap-3">
              {current ? (
                <>
                  <Field label={t("reelTitle")} required error={errors.title && tv("required")}>
                    {({ id, describedBy, invalid }) => <Input id={id} aria-describedby={describedBy} aria-invalid={invalid} value={form.title} onChange={(e) => setForm((f) => ({ ...f, title: e.target.value }))} />}
                  </Field>
                  <div className="grid grid-cols-2 gap-3">
                    <Field label={t("startSec")}>
                      {({ id }) => <Input id={id} type="number" dir="ltr" min={0} value={Math.round(form.start_ms / 1000)} onChange={(e) => setForm((f) => ({ ...f, start_ms: Number(e.target.value) * 1000 }))} />}
                    </Field>
                    <Field label={t("endSec")} error={errors.end_ms && tv("cutLength")}>
                      {({ id, invalid }) => <Input id={id} type="number" dir="ltr" min={0} aria-invalid={invalid} value={Math.round(form.end_ms / 1000)} onChange={(e) => setForm((f) => ({ ...f, end_ms: Number(e.target.value) * 1000 }))} />}
                    </Field>
                  </div>
                  <p className="text-xs text-slate">{t("length", { s: Math.max(0, Math.round((form.end_ms - form.start_ms) / 1000)) })}</p>
                  <Field label={t("topic")} required error={errors.topic_id && tv("required")}>
                    {({ id, invalid }) => (
                      <Select id={id} aria-invalid={invalid} value={form.topic_id} onChange={(e) => setForm((f) => ({ ...f, topic_id: e.target.value }))}>
                        <option value="">—</option>
                        {(topics.data ?? []).map((tp) => (
                          <option key={tp.id} value={tp.id}>
                            {tp.name}
                          </option>
                        ))}
                      </Select>
                    )}
                  </Field>
                  <Checkbox label={t("consent")} description={t("consentHint", { speaker: current.speaker ?? "" })} checked={form.speaker_consent} onChange={(e) => setForm((f) => ({ ...f, speaker_consent: e.target.checked }))} />
                  {errors.speaker_consent ? <p role="alert" className="text-xs font-medium text-ink">{tv("consent")}</p> : null}
                  {cut.error ? <ErrorNotice error={cut.error} /> : null}
                  <Button variant="primary" loading={cut.isPending} onClick={submit} disabled={Boolean(current.reel_id)}>
                    {t("cutButton")}
                  </Button>
                </>
              ) : (
                <p className="text-sm text-slate">{t("pickFirstLong")}</p>
              )}
            </CardBody>
          </Card>
        </aside>
      </div>
    </>
  );
}
