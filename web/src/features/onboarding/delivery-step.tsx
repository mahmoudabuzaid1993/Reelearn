"use client";

import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useTenant } from "@/lib/api/hooks/tenant";
import { ErrorNotice } from "@/components/ui/error-notice";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { DeliverySettingsForm } from "@/features/settings/delivery-settings-form";

export function DeliveryStep() {
  const t = useTranslations("onboarding.delivery");
  const router = useRouter();
  const tenant = useTenant();
  return (
    <>
      <PageHeader eyebrow={t("step")} title={t("title")} description={t("subtitle")} />
      {tenant.isPending ? <SkeletonRows rows={8} /> : tenant.isError ? <ErrorNotice error={tenant.error} onRetry={() => void tenant.refetch()} /> : (
        <DeliverySettingsForm tenant={tenant.data} submitLabel={t("finish")} onSaved={() => router.push("/dashboard")} />
      )}
      <p className="mt-4 text-xs text-slate">{t("channelNote")}</p>
    </>
  );
}
