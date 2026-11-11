"use client";

import { useLocale, useTranslations } from "next-intl";
import type { ReelQuality } from "@/lib/api/types";
import { formatDateTime } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { qualityTone } from "./logic/ai";

/** Compact: the score and whether a reviewer should look closely. */
export function QualityBadge({ quality }: { quality: ReelQuality | null | undefined }) {
  const t = useTranslations("quality");
  if (!quality) return null;
  if (quality.status !== "checked") return <Badge tone="neutral">{t("unavailable")}</Badge>;
  const label = quality.needs_attention ? t("needsAttention") : t("passed");
  return (
    <Badge tone={qualityTone(quality)}>
      <span className="tabular-nums">{t("score", { score: quality.score ?? 0 })}</span>
      <span className="ms-1.5">· {label}</span>
    </Badge>
  );
}

/** The reviewer's view: score, every flag in plain words, and that it is advisory. */
export function QualityCard({ quality }: { quality: ReelQuality | null | undefined }) {
  const t = useTranslations("quality");
  const locale = useLocale();
  if (!quality) return null;
  return (
    <Card data-testid="quality-card">
      <CardHeader title={t("title")} description={t("hint")} actions={<QualityBadge quality={quality} />} />
      <CardBody className="flex flex-col gap-2 text-sm">
        {quality.status !== "checked" ? (
          <p className="text-slate">{quality.message ?? t("unavailable")}</p>
        ) : quality.flags.length === 0 ? (
          <p className="text-slate">{t("noFlags")}</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {quality.flags.map((f) => (
              <li key={f} className="flex items-center gap-2 text-ink">
                <span aria-hidden className="size-1.5 shrink-0 rounded-full bg-ink" />
                {t(`flags.${f}`)}
              </li>
            ))}
          </ul>
        )}
        {quality.checked_at ? <p className="text-xs text-slate">{t("checkedAt", { at: formatDateTime(quality.checked_at, locale) })}</p> : null}
      </CardBody>
    </Card>
  );
}
