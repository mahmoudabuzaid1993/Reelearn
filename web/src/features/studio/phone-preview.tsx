"use client";

import { useTranslations } from "next-intl";
import { directionFor } from "@/i18n/direction";
import type { Reel } from "@/lib/api/types";
import { formatDuration } from "@/lib/format";
import { cn } from "@/lib/cn";

/** A 9:16 phone frame showing what the learner sees: caption line, presenter, the question card. */
export function PhonePreview({ reel, compact = false, captionsOn = true }: { reel: Pick<Reel, "title" | "script" | "language" | "duration_ms" | "question" | "presenter" | "thumbnail_url">; compact?: boolean; captionsOn?: boolean }) {
  const t = useTranslations("studio.preview");
  const dir = directionFor(reel.language);
  const presenter = (reel.presenter as { name?: string } | undefined)?.name;
  const firstLine = (reel.script ?? "").split(/(?<=[.!?؟])\s+/)[0] ?? "";
  return (
    <div className={cn("mx-auto aspect-[9/16] w-full overflow-hidden rounded-[22px] border-[6px] border-night bg-night text-white shadow-lg", compact ? "max-w-[10rem]" : "max-w-[18rem]")} dir={dir} lang={reel.language} aria-label={t("label")} role="img">
      <div className="flex h-full flex-col justify-between bg-[#102F43] p-3">
        <div className="flex items-center justify-between text-[10px] text-white/70">
          <span>{t("todaysReels")}</span>
          <span>{formatDuration(reel.duration_ms)}</span>
        </div>
        <div className="flex flex-1 flex-col items-center justify-center gap-2 text-center">
          <div aria-hidden className={cn("rounded-full bg-white/15", compact ? "size-10" : "size-16")} />
          {presenter ? <p className={cn("text-white/80", compact ? "text-[9px]" : "text-xs")}>{presenter}</p> : null}
        </div>
        <div className="flex flex-col gap-2">
          {captionsOn && firstLine ? (
            <p className={cn("rounded-md bg-black/50 px-2 py-1 text-center leading-snug", compact ? "text-[9px]" : "text-[12px]")}>{firstLine}</p>
          ) : null}
          <p className={cn("font-display font-semibold leading-tight", compact ? "text-[10px]" : "text-sm")}>{reel.title}</p>
          {reel.question && !compact ? (
            <div className="rounded-[10px] bg-white p-2 text-ink">
              <p className="text-[11px] font-medium">{reel.question.prompt}</p>
              <ul className="mt-1 flex flex-col gap-1">
                {reel.question.options.map((o) => (
                  <li key={o.key} className="rounded-md border border-line px-1.5 py-0.5 text-[10px]">
                    <span className="font-semibold text-slate">{o.key}</span> {o.text}
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
