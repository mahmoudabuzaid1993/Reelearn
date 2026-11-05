import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

export type BadgeTone = "neutral" | "ink" | "mint" | "coral" | "outline";

const tones: Record<BadgeTone, string> = {
  neutral: "bg-surface-muted text-slate",
  ink: "bg-ink-soft text-ink",
  mint: "bg-mint-soft text-ink",
  coral: "bg-coral/10 text-ink",
  outline: "border border-line text-slate",
};

export function Badge({ tone = "neutral", children, className }: { tone?: BadgeTone; children: ReactNode; className?: string }) {
  return <span className={cn("inline-flex h-6 items-center whitespace-nowrap rounded-md px-2 text-xs font-medium", tones[tone], className)}>{children}</span>;
}
