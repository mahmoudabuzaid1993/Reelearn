"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import Link from "next/link";
import { useLocale, useTranslations } from "next-intl";
import { useEffect, useState } from "react";
import { useForm, useWatch } from "react-hook-form";
import { useAddVariants, useReel, useUpdateReel } from "@/lib/api/hooks/reels";
import type { Reel } from "@/lib/api/types";
import { formatDateTime, formatDuration } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Checkbox, Input, Select, Switch, Textarea } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { useToast } from "@/components/ui/toast";
import { STATUS_TONE } from "@/features/library/library-view";
import { QualityCard } from "@/features/ai/quality-badge";
import { PhonePreview } from "./phone-preview";
import { reelEditSchema, type ReelEditInput } from "./studio.schema";

const VOICES = ["layla-ar-sa", "faris-ar-sa", "maya-en-gb"];
const LANGS = ["ar", "en", "ur", "bn", "hi", "tl"];

export function ReelEditor({ reelId }: { reelId: string }) {
  const t = useTranslations("studio.editor");
  const tl = useTranslations("library");
  const locale = useLocale();
  const reel = useReel(reelId);
  if (reel.isPending) return <SkeletonRows rows={10} />;
  if (reel.isError) return <ErrorNotice error={reel.error} onRetry={() => void reel.refetch()} />;
  return <EditorForm key={`${reel.data.id}:${reel.data.version}`} reel={reel.data} t={t} tl={tl} locale={locale} />;
}

function toInput(reel: Reel): ReelEditInput {
  const options = reel.question?.options ?? [];
  return {
    title: reel.title,
    script: reel.script ?? "",
    source_citation: reel.source_citation ?? "",
    compliance_tag: reel.compliance_tag ?? "",
    presenter_voice: (reel.presenter as { voice_id?: string } | undefined)?.voice_id ?? "",
    question: {
      prompt: reel.question?.prompt ?? "",
      options: ["A", "B", "C"].map((key, i) => ({ key, text: options[i]?.text ?? "" })),
      correct_key: reel.question?.correct_key ?? "B",
      explanation: reel.question?.explanation ?? "",
    },
  };
}

function EditorForm({ reel, t, tl, locale }: { reel: Reel; t: ReturnType<typeof useTranslations>; tl: ReturnType<typeof useTranslations>; locale: string }) {
  const tv = useTranslations("validation");
  const tc = useTranslations("common");
  const { toast } = useToast();
  const update = useUpdateReel();
  const variants = useAddVariants();
  const [captionsOn, setCaptionsOn] = useState(true);
  const [newLangs, setNewLangs] = useState<string[]>([]);
  const form = useForm<ReelEditInput>({ resolver: zodResolver(reelEditSchema), defaultValues: toInput(reel) });
  const watched = useWatch({ control: form.control });
  const err = form.formState.errors;

  useEffect(() => {
    form.reset(toInput(reel));
  }, [reel, form]);

  const preview = {
    title: watched.title ?? reel.title,
    script: watched.script ?? reel.script ?? "",
    language: reel.language,
    duration_ms: reel.duration_ms ?? null,
    presenter: { name: (reel.presenter as { name?: string } | undefined)?.name } as never,
    thumbnail_url: null,
    question: watched.question?.prompt
      ? { id: reel.question?.id ?? "", language: reel.language, prompt: watched.question.prompt, options: (watched.question.options ?? []).map((o) => ({ key: o?.key ?? "", text: o?.text ?? "" })) }
      : null,
  };

  const save = form.handleSubmit(async (values) => {
    await update.mutateAsync({
      id: reel.id,
      body: {
        title: values.title,
        script: values.script,
        source_citation: values.source_citation || undefined,
        compliance_tag: values.compliance_tag || undefined,
        presenter: { ...(reel.presenter as object), voice_id: values.presenter_voice || undefined } as never,
        question: { prompt: values.question.prompt, options: values.question.options, correct_key: values.question.correct_key, explanation: values.question.explanation || undefined },
      },
    });
    toast({ title: reel.status === "published" ? t("savedNewVersion") : t("saved"), tone: "success" });
  });

  const existingLangs = [reel.language, ...(reel.variants ?? []).map((v) => v.language)];

  return (
    <>
      <PageHeader
        eyebrow={t("step")}
        title={reel.title}
        description={
          <span className="flex flex-wrap items-center gap-2">
            <Badge tone={STATUS_TONE[reel.status]}>{tl(`statuses.${reel.status}`)}</Badge>
            <span>
              v{reel.version} · {tl(`kinds.${reel.kind}`)} · {formatDuration(reel.duration_ms)} · {t("updated", { at: formatDateTime(reel.updated_at, locale) })}
            </span>
          </span>
        }
        actions={
          <>
            <Button variant="secondary" loading={update.isPending} onClick={() => void save()}>
              {tc("save")}
            </Button>
            <Link href={`/studio/reels/${reel.id}/publish`} className="inline-flex h-10 items-center rounded-[10px] border border-coral bg-coral px-4 font-display text-sm font-medium text-white hover:bg-coral-strong">
              {t("goToPublish")}
            </Link>
          </>
        }
      />
      {reel.status === "published" ? <p className="mb-4 rounded-[10px] border border-line bg-surface px-4 py-2 text-sm text-slate">{t("publishedNote")}</p> : null}

      <form onSubmit={save} noValidate className="grid grid-cols-1 gap-4 lg:grid-cols-[1fr_20rem]">
        <div className="flex flex-col gap-4">
          {reel.status !== "published" ? <QualityCard quality={reel.quality} /> : null}
          <Card>
            <CardHeader title={t("script")} description={t("scriptHint")} />
            <CardBody className="flex flex-col gap-3">
              <Field label={t("title")} required error={err.title && tv("required")}>
                {({ id, describedBy, invalid }) => <Input id={id} aria-describedby={describedBy} aria-invalid={invalid} {...form.register("title")} />}
              </Field>
              <Field label={t("scriptText")} required hint={t("scriptCount", { n: (watched.script ?? "").length })} error={err.script && tv(err.script.message === "scriptMax" ? "scriptMax" : "script")}>
                {({ id, describedBy, invalid }) => <Textarea id={id} rows={7} aria-describedby={describedBy} aria-invalid={invalid} {...form.register("script")} />}
              </Field>
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
                <Field label={t("citation")} hint={t("citationHint")}>{({ id, describedBy }) => <Input id={id} aria-describedby={describedBy} {...form.register("source_citation")} />}</Field>
                <Field label={t("complianceTag")}>{({ id }) => <Input id={id} dir="ltr" placeholder="HRDF-SAF" {...form.register("compliance_tag")} />}</Field>
              </div>
            </CardBody>
          </Card>

          <Card>
            <CardHeader title={t("voiceVisual")} />
            <CardBody className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <Field label={t("voice")}>
                {({ id }) => (
                  <Select id={id} {...form.register("presenter_voice")}>
                    <option value="">{t("keepVoice")}</option>
                    {VOICES.map((v) => (
                      <option key={v} value={v}>
                        {v}
                      </option>
                    ))}
                  </Select>
                )}
              </Field>
              <div className="flex flex-col justify-end">
                <Switch checked={captionsOn} onChange={setCaptionsOn} label={t("captions")} description={t("captionsHint")} />
              </div>
              <fieldset className="sm:col-span-2">
                <legend className="mb-1.5 text-[13px] font-medium text-ink">{t("variants")}</legend>
                <div className="flex flex-wrap items-center gap-3">
                  {LANGS.map((l) =>
                    existingLangs.includes(l) ? (
                      <Badge key={l} tone="mint">
                        {tc(`languages.${l}`)} · {(reel.variants ?? []).find((v) => v.language === l)?.dub_status ?? t("original")}
                      </Badge>
                    ) : (
                      <Checkbox key={l} label={tc(`languages.${l}`)} checked={newLangs.includes(l)} onChange={(e) => setNewLangs((p) => (e.target.checked ? [...p, l] : p.filter((x) => x !== l)))} />
                    ),
                  )}
                  <Button
                    size="sm"
                    disabled={newLangs.length === 0}
                    loading={variants.isPending}
                    onClick={() =>
                      variants.mutate(
                        { id: reel.id, languages: newLangs },
                        {
                          onSuccess: () => {
                            toast({ title: t("variantsQueued", { n: newLangs.length }), description: t("variantsCost", { n: newLangs.length * 5 }) });
                            setNewLangs([]);
                          },
                        },
                      )
                    }
                  >
                    {t("addVariants")}
                  </Button>
                </div>
              </fieldset>
            </CardBody>
          </Card>

          <Card>
            <CardHeader title={t("question")} description={t("questionHint")} />
            <CardBody className="flex flex-col gap-3">
              <Field label={t("prompt")} required error={err.question?.prompt && tv("required")}>
                {({ id, describedBy, invalid }) => <Input id={id} aria-describedby={describedBy} aria-invalid={invalid} {...form.register("question.prompt")} />}
              </Field>
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
                {(["A", "B", "C"] as const).map((key, i) => (
                  <Field key={key} label={t("option", { key })} required error={err.question?.options?.[i]?.text && tv("required")}>
                    {({ id, describedBy, invalid }) => (
                      <span className="flex items-center gap-2">
                        <input type="radio" aria-label={t("correctIs", { key })} value={key} className="accent-ink" {...form.register("question.correct_key")} />
                        <Input id={id} aria-describedby={describedBy} aria-invalid={invalid} {...form.register(`question.options.${i}.text` as const)} />
                      </span>
                    )}
                  </Field>
                ))}
              </div>
              <p className="text-xs text-slate">{t("correctHint")}</p>
              <Field label={t("explanation")} hint={t("explanationHint")}>{({ id, describedBy }) => <Input id={id} aria-describedby={describedBy} {...form.register("question.explanation")} />}</Field>
            </CardBody>
          </Card>
          {update.error ? <ErrorNotice error={update.error} /> : null}
        </div>

        <aside className="lg:sticky lg:top-6 lg:self-start">
          <Card>
            <CardHeader title={t("preview")} description={t("previewHint")} />
            <CardBody>
              <PhonePreview reel={preview} captionsOn={captionsOn} />
            </CardBody>
          </Card>
        </aside>
      </form>
    </>
  );
}
