"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { useTranslations } from "next-intl";
import { useRouter, useSearchParams } from "next/navigation";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useRequestOtp, useVerifyOtp } from "@/lib/api/hooks/auth";
import { Button } from "@/components/ui/button";
import { Card, CardBody } from "@/components/ui/card";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { isMockMode } from "@/lib/auth/config";
import { otpSchema, signInSchema, type OtpInput, type SignInInput } from "./auth.schema";

interface Challenge {
  id: string;
  email: string;
  channel: string;
}

export function SignInForm({ initialSlug = "alwadi", initialChallengeId }: { initialSlug?: string; initialChallengeId?: string | null }) {
  const t = useTranslations("auth");
  const tv = useTranslations("validation");
  const router = useRouter();
  const params = useSearchParams();
  const next = params.get("next");
  const [challenge, setChallenge] = useState<Challenge | null>(initialChallengeId ? { id: initialChallengeId, email: "", channel: "email" } : null);
  const requestOtp = useRequestOtp();
  const verifyOtp = useVerifyOtp();

  const form = useForm<SignInInput>({ resolver: zodResolver(signInSchema), defaultValues: { tenant_slug: initialSlug, email: "" } });
  const otpForm = useForm<OtpInput>({ resolver: zodResolver(otpSchema), defaultValues: { code: "" } });

  const onRequest = form.handleSubmit(async (values) => {
    const res = await requestOtp.mutateAsync({ tenant_slug: values.tenant_slug, email: values.email });
    setChallenge({ id: res.challenge_id, email: values.email, channel: res.channel });
  });

  const onVerify = otpForm.handleSubmit(async (values) => {
    await verifyOtp.mutateAsync({ challenge_id: challenge!.id, code: values.code });
    router.replace(next && next.startsWith("/") ? next : "/dashboard");
  });

  return (
    <Card>
      <CardBody className="flex flex-col gap-5 py-6">
        <div>
          <h1 className="font-display text-xl font-semibold text-ink">{t("title")}</h1>
          <p className="mt-1 text-sm text-slate">{challenge ? t("codeSentTo", { email: challenge.email || t("yourEmail") }) : t("subtitle")}</p>
        </div>

        {!challenge ? (
          <form onSubmit={onRequest} noValidate className="flex flex-col gap-4">
            <Field label={t("workspace")} hint={t("workspaceHint")} error={form.formState.errors.tenant_slug && tv("slug")}>
              {({ id, describedBy, invalid }) => (
                <Input id={id} aria-describedby={describedBy} aria-invalid={invalid} autoComplete="organization" dir="ltr" {...form.register("tenant_slug")} />
              )}
            </Field>
            <Field label={t("email")} error={form.formState.errors.email && tv("email")}>
              {({ id, describedBy, invalid }) => (
                <Input id={id} type="email" inputMode="email" autoComplete="email" dir="ltr" aria-describedby={describedBy} aria-invalid={invalid} {...form.register("email")} />
              )}
            </Field>
            {requestOtp.error ? <ErrorNotice error={requestOtp.error} /> : null}
            <Button type="submit" variant="primary" loading={requestOtp.isPending}>
              {t("sendCode")}
            </Button>
            {isMockMode() ? <p className="text-xs text-slate">{t("mockHint")}</p> : null}
          </form>
        ) : (
          <form onSubmit={onVerify} noValidate className="flex flex-col gap-4">
            <Field label={t("code")} hint={isMockMode() ? t("devCodeHint") : undefined} error={otpForm.formState.errors.code && tv("code")}>
              {({ id, describedBy, invalid }) => (
                <Input id={id} inputMode="numeric" autoComplete="one-time-code" dir="ltr" maxLength={8} aria-describedby={describedBy} aria-invalid={invalid} {...otpForm.register("code")} />
              )}
            </Field>
            {verifyOtp.error ? <ErrorNotice error={verifyOtp.error} /> : null}
            <Button type="submit" variant="primary" loading={verifyOtp.isPending}>
              {t("verify")}
            </Button>
            <Button type="button" variant="quiet" onClick={() => setChallenge(null)}>
              {t("useAnotherEmail")}
            </Button>
          </form>
        )}
      </CardBody>
    </Card>
  );
}
