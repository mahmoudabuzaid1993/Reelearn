"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useForm } from "react-hook-form";
import { useCreateTenant } from "@/lib/api/hooks/tenant";
import { isApiError } from "@/lib/api/errors";
import { Button } from "@/components/ui/button";
import { Card, CardBody } from "@/components/ui/card";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Input, Select } from "@/components/ui/input";
import { signUpSchema, suggestSlug, type SignUpInput } from "./auth.schema";

const SECTORS = ["retail", "logistics", "hospitality", "banking", "healthcare", "other"] as const;

export function SignUpForm() {
  const t = useTranslations("signup");
  const tv = useTranslations("validation");
  const router = useRouter();
  const create = useCreateTenant();
  const form = useForm<SignUpInput>({
    resolver: zodResolver(signUpSchema),
    defaultValues: { company_name: "", slug: "", admin_name: "", admin_email: "", admin_phone: "", default_language: "ar", region: "ksa", sector: "retail" },
  });
  const err = form.formState.errors;
  const slugTaken = isApiError(create.error) && create.error.fieldErrors.slug;

  const onSubmit = form.handleSubmit(async (values) => {
    const res = await create.mutateAsync({ ...values, admin_phone: values.admin_phone || undefined, sector: values.sector || undefined });
    router.push(`/signin?challenge=${encodeURIComponent(res.challenge_id ?? "")}&slug=${encodeURIComponent(values.slug)}`);
  });

  return (
    <Card>
      <CardBody className="py-6">
        <h1 className="font-display text-xl font-semibold text-ink">{t("title")}</h1>
        <p className="mt-1 text-sm text-slate">{t("subtitle")}</p>
        <form onSubmit={onSubmit} noValidate className="mt-5 flex flex-col gap-4">
          <Field label={t("companyName")} required error={err.company_name && tv("required")}>
            {({ id, describedBy, invalid }) => (
              <Input
                id={id}
                aria-describedby={describedBy}
                aria-invalid={invalid}
                autoComplete="organization"
                {...form.register("company_name", {
                  onChange: (e) => {
                    if (!form.formState.dirtyFields.slug) form.setValue("slug", suggestSlug(e.target.value));
                  },
                })}
              />
            )}
          </Field>
          <Field label={t("slug")} required hint={t("slugHint")} error={(err.slug && tv("slug")) || (slugTaken ? t("slugTaken") : null)}>
            {({ id, describedBy, invalid }) => <Input id={id} dir="ltr" aria-describedby={describedBy} aria-invalid={invalid} {...form.register("slug")} />}
          </Field>
          <Field label={t("adminName")} required error={err.admin_name && tv("required")}>
            {({ id, describedBy, invalid }) => <Input id={id} autoComplete="name" aria-describedby={describedBy} aria-invalid={invalid} {...form.register("admin_name")} />}
          </Field>
          <Field label={t("adminEmail")} required error={err.admin_email && tv("email")}>
            {({ id, describedBy, invalid }) => <Input id={id} type="email" dir="ltr" autoComplete="email" aria-describedby={describedBy} aria-invalid={invalid} {...form.register("admin_email")} />}
          </Field>
          <Field label={t("adminPhone")} hint={t("adminPhoneHint")} error={err.admin_phone && tv("phone")}>
            {({ id, describedBy, invalid }) => <Input id={id} type="tel" dir="ltr" autoComplete="tel" aria-describedby={describedBy} aria-invalid={invalid} {...form.register("admin_phone")} />}
          </Field>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
            <Field label={t("language")} required>
              {({ id }) => (
                <Select id={id} {...form.register("default_language")}>
                  <option value="ar">العربية</option>
                  <option value="en">English</option>
                </Select>
              )}
            </Field>
            <Field label={t("region")} required hint={t("regionHint")}>
              {({ id, describedBy }) => (
                <Select id={id} aria-describedby={describedBy} {...form.register("region")}>
                  <option value="ksa">{t("regions.ksa")}</option>
                  <option value="uae">{t("regions.uae")}</option>
                  <option value="eu">{t("regions.eu")}</option>
                </Select>
              )}
            </Field>
            <Field label={t("sector")}>
              {({ id }) => (
                <Select id={id} {...form.register("sector")}>
                  {SECTORS.map((s) => (
                    <option key={s} value={s}>
                      {t(`sectors.${s}`)}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
          </div>
          {create.error && !slugTaken ? <ErrorNotice error={create.error} /> : null}
          <Button type="submit" variant="primary" loading={create.isPending}>
            {t("create")}
          </Button>
          <p className="text-xs text-slate">{t("privacyNote")}</p>
        </form>
      </CardBody>
    </Card>
  );
}
