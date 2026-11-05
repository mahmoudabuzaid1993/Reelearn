"use client";

import { useLocale, useTranslations } from "next-intl";
import type { Dashboard } from "@/lib/api/types";

type Point = NonNullable<Dashboard["series"]>[number];

/** Pure SVG chart: bars for active learners, a line for recall. No gradients, brand colours only. */
export function ActivityChart({ series }: { series: Point[] }) {
  const t = useTranslations("dashboard");
  const locale = useLocale();
  const width = 640;
  const height = 180;
  const pad = { top: 10, bottom: 24, start: 28, end: 8 };
  const innerW = width - pad.start - pad.end;
  const innerH = height - pad.top - pad.bottom;
  const maxActive = Math.max(1, ...series.map((p) => p.active ?? 0));
  const barW = innerW / series.length;

  const x = (i: number) => pad.start + i * barW;
  const yActive = (v: number) => pad.top + innerH - (v / maxActive) * innerH;
  const yRecall = (v: number) => pad.top + innerH - (v / 100) * innerH;
  const line = series.map((p, i) => `${i === 0 ? "M" : "L"}${(x(i) + barW / 2).toFixed(1)},${yRecall(p.recall_pct ?? 0).toFixed(1)}`).join(" ");
  const fmt = new Intl.DateTimeFormat(locale, { day: "numeric", month: "short" });

  return (
    <figure>
      <svg viewBox={`0 0 ${width} ${height}`} role="img" aria-label={t("chartLabel")} className="w-full" style={{ direction: "ltr" }}>
        <line x1={pad.start} x2={width - pad.end} y1={pad.top + innerH} y2={pad.top + innerH} stroke="#DDE2E0" />
        {[0, 50, 100].map((v) => (
          <g key={v}>
            <text x={pad.start - 6} y={yRecall(v) + 4} textAnchor="end" fontSize="10" fill="#5B6B78">
              {v}%
            </text>
            <line x1={pad.start} x2={width - pad.end} y1={yRecall(v)} y2={yRecall(v)} stroke="#DDE2E0" strokeDasharray="2 4" />
          </g>
        ))}
        {series.map((p, i) => (
          <rect key={p.day} x={x(i) + barW * 0.2} y={yActive(p.active ?? 0)} width={barW * 0.6} height={pad.top + innerH - yActive(p.active ?? 0)} fill="#DDE2E0" rx="2">
            <title>{`${p.day}: ${p.active} ${t("active")}`}</title>
          </rect>
        ))}
        <path d={line} fill="none" stroke="#2EC4B6" strokeWidth="2" />
        {series.filter((_, i) => i % 7 === 0).map((p) => (
          <text key={p.day} x={x(series.indexOf(p)) + barW / 2} y={height - 6} textAnchor="middle" fontSize="10" fill="#5B6B78">
            {fmt.format(new Date(p.day ?? ""))}
          </text>
        ))}
      </svg>
      <figcaption className="mt-2 flex gap-4 text-xs text-slate">
        <span className="inline-flex items-center gap-1.5">
          <span aria-hidden className="inline-block h-2.5 w-3 rounded-sm bg-line" /> {t("activeLearners")}
        </span>
        <span className="inline-flex items-center gap-1.5">
          <span aria-hidden className="inline-block h-0.5 w-3 bg-mint" /> {t("recallLine")}
        </span>
      </figcaption>
    </figure>
  );
}
