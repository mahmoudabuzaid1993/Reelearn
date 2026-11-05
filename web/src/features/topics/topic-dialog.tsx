"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { useTranslations } from "next-intl";
import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { useCreateTopic, useUpdateTopic } from "@/lib/api/hooks/topics";
import type { Topic } from "@/lib/api/types";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Checkbox, Input, Textarea } from "@/components/ui/input";
import { useToast } from "@/components/ui/toast";
import { topicSchema, type TopicInput } from "./topics.schema";

export function TopicDialog({ topic, open, onClose }: { topic: Topic | null; open: boolean; onClose: () => void }) {
  const t = useTranslations("topics.dialog");
  const tc = useTranslations("common");
  const tv = useTranslations("validation");
  const create = useCreateTopic();
  const update = useUpdateTopic();
  const { toast } = useToast();
  const form = useForm<TopicInput>({ resolver: zodResolver(topicSchema), defaultValues: { name: "", name_ar: "", description: "", compliance_tag: "", is_mandatory: false } });

  useEffect(() => {
    form.reset({
      name: topic?.name ?? "",
      name_ar: topic?.name_ar ?? "",
      description: topic?.description ?? "",
      compliance_tag: topic?.compliance_tag ?? "",
      is_mandatory: topic?.is_mandatory ?? false,
    });
  }, [topic, open, form]);

  const submit = form.handleSubmit(async (values) => {
    const body = { ...values, name_ar: values.name_ar || undefined, description: values.description || undefined, compliance_tag: values.compliance_tag || undefined };
    if (topic) await update.mutateAsync({ id: topic.id, body });
    else await create.mutateAsync(body);
    toast({ title: t("saved"), tone: "success" });
    onClose();
  });
  const pending = create.isPending || update.isPending;
  const error = create.error ?? update.error;

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={topic ? t("editTitle") : t("newTitle")}
      footer={
        <>
          <Button onClick={onClose}>{tc("cancel")}</Button>
          <Button variant="primary" loading={pending} onClick={() => void submit()}>
            {tc("save")}
          </Button>
        </>
      }
    >
      <form onSubmit={submit} noValidate className="flex flex-col gap-3">
        <Field label={t("name")} required error={form.formState.errors.name && tv("required")}>
          {({ id, describedBy, invalid }) => <Input id={id} aria-describedby={describedBy} aria-invalid={invalid} {...form.register("name")} />}
        </Field>
        <Field label={t("nameAr")}>{({ id }) => <Input id={id} dir="rtl" lang="ar" {...form.register("name_ar")} />}</Field>
        <Field label={t("description")}>{({ id }) => <Textarea id={id} {...form.register("description")} />}</Field>
        <Field label={t("complianceTag")} hint={t("complianceHint")}>
          {({ id, describedBy }) => <Input id={id} dir="ltr" aria-describedby={describedBy} placeholder="HRDF-SAF" {...form.register("compliance_tag")} />}
        </Field>
        <Checkbox label={t("mandatory")} description={t("mandatoryHint")} {...form.register("is_mandatory")} />
        {error ? <ErrorNotice error={error} /> : null}
      </form>
    </Dialog>
  );
}
