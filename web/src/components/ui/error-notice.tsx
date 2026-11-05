"use client";

import { useTranslations } from "next-intl";
import { useState } from "react";
import { isApiError } from "@/lib/api/errors";
import { Button } from "./button";

/** Inline, human error with the request id for support. Never red; ink on paper. */
export function ErrorNotice({ error, onRetry }: { error: unknown; onRetry?: () => void }) {
  const t = useTranslations("errors");
  const [copied, setCopied] = useState(false);
  const apiError = isApiError(error) ? error : null;
  const message = apiError?.message ?? (error instanceof Error ? error.message : t("generic"));
  const requestId = apiError?.requestId;

  return (
    <div role="alert" className="flex flex-col gap-2 rounded-[14px] border border-ink/30 bg-surface px-4 py-3 text-sm">
      <p className="font-medium text-ink">{apiError?.status === 401 ? t("unauthorized") : message}</p>
      <div className="flex flex-wrap items-center gap-3 text-xs text-slate">
        {requestId ? (
          <span className="inline-flex items-center gap-2">
            {t("requestId")}: <code className="rounded bg-surface-muted px-1.5 py-0.5">{requestId}</code>
            <button
              type="button"
              className="underline underline-offset-2"
              onClick={() => {
                void navigator.clipboard?.writeText(requestId);
                setCopied(true);
              }}
            >
              {copied ? t("copied") : t("copy")}
            </button>
          </span>
        ) : null}
        {onRetry ? (
          <Button size="sm" onClick={onRetry}>
            {t("retry")}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
