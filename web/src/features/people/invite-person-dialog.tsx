"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { useTranslations } from "next-intl";
import { useForm } from "react-hook-form";
import { useGroups, useInvitePerson } from "@/lib/api/hooks/people";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Checkbox, Input, Select } from "@/components/ui/input";
import { useToast } from "@/components/ui/toast";
import { invitePersonSchema, type InvitePersonInput } from "./people.schema";

export function InvitePersonDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const t = useTranslations("people");
  const tc = useTranslations("common");
  const tv = useTranslations("validation");
  const groups = useGroups();
  const invite = useInvitePerson();
  const { toast } = useToast();
  const form = useForm<InvitePersonInput>({
    resolver: zodResolver(invitePersonSchema),
    defaultValues: { full_name: "", role: "learner", email: "", phone: "", preferred_language: "ar", delivery_channel: "app", group_ids: [] },
  });
  const err = form.formState.errors;

  const submit = form.handleSubmit(async (values) => {
    await invite.mutateAsync({ ...values, email: values.email || undefined, phone: values.phone || undefined, send_invite: true });
    toast({ title: t("invited", { name: values.full_name }), tone: "success" });
    form.reset();
    onClose();
  });

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={t("invite")}
      description={t("inviteHint")}
      footer={
        <>
          <Button onClick={onClose}>{tc("cancel")}</Button>
          <Button variant="primary" loading={invite.isPending} onClick={() => void submit()}>
            {t("sendInvite")}
          </Button>
        </>
      }
    >
      <form onSubmit={submit} noValidate className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <Field label={t("name")} required error={err.full_name && tv("required")} className="sm:col-span-2">
          {({ id, describedBy, invalid }) => <Input id={id} aria-describedby={describedBy} aria-invalid={invalid} {...form.register("full_name")} />}
        </Field>
        <Field label={t("role")}>
          {({ id }) => (
            <Select id={id} {...form.register("role")}>
              {(["learner", "manager", "sme", "admin"] as const).map((r) => (
                <option key={r} value={r}>
                  {tc(`roles.${r}`)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t("language")}>
          {({ id }) => (
            <Select id={id} {...form.register("preferred_language")}>
              {(["ar", "en", "ur", "bn", "hi", "tl"] as const).map((l) => (
                <option key={l} value={l}>
                  {tc(`languages.${l}`)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t("phone")} hint={t("phoneHint")} error={err.phone && tv(err.phone.message === "contact" ? "contact" : "phone")}>
          {({ id, describedBy, invalid }) => <Input id={id} type="tel" dir="ltr" aria-describedby={describedBy} aria-invalid={invalid} {...form.register("phone")} />}
        </Field>
        <Field label={t("email")} error={err.email && tv("email")}>
          {({ id, describedBy, invalid }) => <Input id={id} type="email" dir="ltr" aria-describedby={describedBy} aria-invalid={invalid} {...form.register("email")} />}
        </Field>
        <Field label={t("channel")}>
          {({ id }) => (
            <Select id={id} {...form.register("delivery_channel")}>
              {(["app", "whatsapp", "both", "teams"] as const).map((c) => (
                <option key={c} value={c}>
                  {tc(`channels.${c}`)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t("employeeNo")}>{({ id }) => <Input id={id} dir="ltr" {...form.register("employee_no")} />}</Field>
        <fieldset className="sm:col-span-2">
          <legend className="mb-1.5 text-[13px] font-medium text-ink">{t("groups")}</legend>
          <div className="flex flex-wrap gap-3">
            {(groups.data ?? []).map((g) => (
              <Checkbox key={g.id} label={g.name} value={g.id} {...form.register("group_ids")} />
            ))}
          </div>
        </fieldset>
        {invite.error ? (
          <div className="sm:col-span-2">
            <ErrorNotice error={invite.error} />
          </div>
        ) : null}
      </form>
    </Dialog>
  );
}
