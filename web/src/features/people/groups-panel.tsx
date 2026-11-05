"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useCreateGroup, useGroups } from "@/lib/api/hooks/people";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardHeader } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Input, Select } from "@/components/ui/input";
import { SkeletonRows } from "@/components/ui/skeleton";
import { useToast } from "@/components/ui/toast";
import { groupSchema, type GroupInput } from "./people.schema";

export function GroupsPanel() {
  const t = useTranslations("people.groupsPanel");
  const tc = useTranslations("common");
  const tv = useTranslations("validation");
  const groups = useGroups();
  const create = useCreateGroup();
  const { toast } = useToast();
  const [open, setOpen] = useState(false);
  const form = useForm<GroupInput>({ resolver: zodResolver(groupSchema), defaultValues: { name: "", kind: "team" } });

  const submit = form.handleSubmit(async (values) => {
    await create.mutateAsync(values);
    toast({ title: t("created", { name: values.name }), tone: "success" });
    form.reset();
    setOpen(false);
  });

  return (
    <Card>
      <CardHeader title={t("title")} description={t("hint")} actions={<Button variant="primary" onClick={() => setOpen(true)}>{t("newGroup")}</Button>} />
      {groups.isPending ? (
        <div className="p-5">
          <SkeletonRows rows={3} />
        </div>
      ) : groups.isError ? (
        <div className="p-5">
          <ErrorNotice error={groups.error} onRetry={() => void groups.refetch()} />
        </div>
      ) : groups.data.length === 0 ? (
        <div className="p-5">
          <EmptyState title={t("empty")} />
        </div>
      ) : (
        <ul className="divide-y divide-line">
          {groups.data.map((g) => (
            <li key={g.id} className="flex items-center justify-between px-5 py-3 text-sm">
              <div>
                <p className="font-medium text-ink">{g.name}</p>
                <p className="text-xs text-slate">{t(`kinds.${g.kind}`)}</p>
              </div>
              <Badge tone="ink">{t("members", { n: g.member_count })}</Badge>
            </li>
          ))}
        </ul>
      )}
      <Dialog
        open={open}
        onClose={() => setOpen(false)}
        title={t("newGroup")}
        footer={
          <>
            <Button onClick={() => setOpen(false)}>{tc("cancel")}</Button>
            <Button variant="primary" loading={create.isPending} onClick={() => void submit()}>
              {tc("create")}
            </Button>
          </>
        }
      >
        <form onSubmit={submit} noValidate className="flex flex-col gap-3">
          <Field label={t("name")} required error={form.formState.errors.name && tv("required")}>
            {({ id, describedBy, invalid }) => <Input id={id} aria-describedby={describedBy} aria-invalid={invalid} {...form.register("name")} />}
          </Field>
          <Field label={t("kind")}>
            {({ id }) => (
              <Select id={id} {...form.register("kind")}>
                {(["team", "location", "role", "custom"] as const).map((k) => (
                  <option key={k} value={k}>
                    {t(`kinds.${k}`)}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          {create.error ? <ErrorNotice error={create.error} /> : null}
        </form>
      </Dialog>
    </Card>
  );
}
