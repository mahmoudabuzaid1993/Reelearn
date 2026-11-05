"use client";

import { useLocale, useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useTransition } from "react";
import { otherLocale } from "@/i18n/direction";
import { setLocaleAction } from "@/i18n/set-locale";
import type { Locale } from "@/i18n/locales";
import { Button } from "@/components/ui/button";

export function LocaleSwitch() {
  const locale = useLocale() as Locale;
  const t = useTranslations("common");
  const router = useRouter();
  const [pending, startTransition] = useTransition();
  const next = otherLocale(locale);
  return (
    <Button
      size="sm"
      variant="quiet"
      lang={next}
      aria-label={t("switchLocale", { locale: next })}
      disabled={pending}
      onClick={() =>
        startTransition(async () => {
          await setLocaleAction(next);
          router.refresh();
        })
      }
    >
      {next === "ar" ? "العربية" : "English"}
    </Button>
  );
}
