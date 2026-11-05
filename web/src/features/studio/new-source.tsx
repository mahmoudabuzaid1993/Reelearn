"use client";

import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { useCreateSource, useRequestUpload } from "@/lib/api/hooks/studio";
import { ApiError, isApiError } from "@/lib/api/errors";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Input, Select, Textarea } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { Tabs } from "@/components/ui/tabs";
import { kindForFile, MAX_UPLOAD_BYTES, sourceSchema, type SourceInput } from "./studio.schema";

type Mode = "file" | "url" | "youtube" | "prompt";

export function NewSource() {
  const t = useTranslations("studio.newSource");
  const tv = useTranslations("validation");
  const router = useRouter();
  const [mode, setMode] = useState<Mode>("file");
  const [title, setTitle] = useState("");
  const [url, setUrl] = useState("");
  const [prompt, setPrompt] = useState("");
  const [language, setLanguage] = useState("ar");
  const [file, setFile] = useState<File | null>(null);
  const [fileError, setFileError] = useState<string | null>(null);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const upload = useRequestUpload();
  const create = useCreateSource();
  const [uploading, setUploading] = useState(false);

  const onFile = (f: File | null) => {
    setFile(null);
    setFileError(null);
    if (!f) return;
    if (!kindForFile(f)) return setFileError(t("unsupported"));
    if (f.size > MAX_UPLOAD_BYTES) return setFileError(t("tooLarge"));
    setFile(f);
    if (!title) setTitle(f.name.replace(/\.[^.]+$/, ""));
  };

  const submit = async () => {
    setErrors({});
    let input: SourceInput;
    if (mode === "file") {
      if (!file) return setFileError(t("fileRequired"));
      const kind = kindForFile(file)!;
      setUploading(true);
      try {
        const slot = await upload.mutateAsync({ filename: file.name, mime: file.type || "application/octet-stream", bytes: file.size });
        await fetch(slot.upload_url, { method: slot.method, headers: slot.headers, body: file });
        input = { kind: kind as "pdf", title, media_asset_id: slot.id, language };
      } finally {
        setUploading(false);
      }
    } else if (mode === "url") input = { kind: "url", title, url, language };
    else if (mode === "youtube") input = { kind: "youtube", title, url, language };
    else input = { kind: "topic_prompt", title, prompt, language };

    const parsed = sourceSchema.safeParse(input);
    if (!parsed.success) {
      setErrors(Object.fromEntries(parsed.error.issues.map((i) => [String(i.path[0]), i.message])));
      return;
    }
    const source = await create.mutateAsync(parsed.data);
    router.push(source.kind === "podcast_audio" || source.kind === "video" ? `/studio/sources/${source.id}/moments` : `/studio/jobs/new?source=${source.id}`);
  };

  const apiFieldError = (field: string) => (isApiError(create.error) ? create.error.fieldErrors[field] : undefined);

  return (
    <>
      <PageHeader eyebrow={t("step")} title={t("title")} description={t("subtitle")} />
      <Card>
        <CardHeader title={t("pick")} />
        <CardBody className="flex flex-col gap-5">
          <Tabs ariaLabel={t("pick")} value={mode} onChange={setMode} items={[{ value: "file", label: t("modes.file") }, { value: "url", label: t("modes.url") }, { value: "youtube", label: t("modes.youtube") }, { value: "prompt", label: t("modes.prompt") }]} />

          {mode === "file" ? (
            <Field label={t("file")} hint={t("fileHint")} error={fileError}>
              {({ id, describedBy, invalid }) => (
                <input
                  id={id}
                  type="file"
                  accept=".pdf,.pptx,.png,.jpg,.jpeg,.mp3,.m4a,.wav,.mp4,.mov"
                  aria-describedby={describedBy}
                  aria-invalid={invalid}
                  className="block w-full text-sm text-slate file:me-3 file:rounded-md file:border file:border-line file:bg-surface file:px-3 file:py-1.5 file:text-sm file:text-ink"
                  onChange={(e) => onFile(e.target.files?.[0] ?? null)}
                />
              )}
            </Field>
          ) : null}
          {mode === "url" || mode === "youtube" ? (
            <Field label={mode === "youtube" ? t("youtubeUrl") : t("url")} hint={mode === "youtube" ? t("youtubeHint") : t("urlHint")} error={(errors.url && tv(errors.url)) || apiFieldError("url")}>
              {({ id, describedBy, invalid }) => <Input id={id} type="url" dir="ltr" aria-describedby={describedBy} aria-invalid={invalid} value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://" />}
            </Field>
          ) : null}
          {mode === "prompt" ? (
            <Field label={t("prompt")} hint={t("promptHint")} error={errors.prompt && tv(errors.prompt)}>
              {({ id, describedBy, invalid }) => <Textarea id={id} aria-describedby={describedBy} aria-invalid={invalid} value={prompt} onChange={(e) => setPrompt(e.target.value)} />}
            </Field>
          ) : null}

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-[1fr_12rem]">
            <Field label={t("sourceTitle")} required error={errors.title && tv(errors.title)}>
              {({ id, describedBy, invalid }) => <Input id={id} aria-describedby={describedBy} aria-invalid={invalid} value={title} onChange={(e) => setTitle(e.target.value)} />}
            </Field>
            <Field label={t("language")}>
              {({ id }) => (
                <Select id={id} value={language} onChange={(e) => setLanguage(e.target.value)}>
                  <option value="ar">العربية</option>
                  <option value="en">English</option>
                </Select>
              )}
            </Field>
          </div>

          {create.error && !apiFieldError("url") ? <ErrorNotice error={create.error} /> : null}
          {upload.error ? <ErrorNotice error={upload.error instanceof ApiError ? upload.error : new Error(t("uploadFailed"))} /> : null}
          <div className="flex justify-end">
            <Button variant="primary" loading={create.isPending || uploading} onClick={() => void submit()}>
              {t("continue")}
            </Button>
          </div>
        </CardBody>
      </Card>
    </>
  );
}
