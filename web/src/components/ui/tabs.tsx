"use client";

import { cn } from "@/lib/cn";

export interface TabItem<T extends string> {
  value: T;
  label: React.ReactNode;
  count?: number;
}

export function Tabs<T extends string>({ items, value, onChange, ariaLabel }: { items: TabItem<T>[]; value: T; onChange: (v: T) => void; ariaLabel: string }) {
  return (
    <div role="tablist" aria-label={ariaLabel} className="flex gap-1 border-b border-line">
      {items.map((item) => {
        const selected = item.value === value;
        return (
          <button
            key={item.value}
            role="tab"
            type="button"
            aria-selected={selected}
            tabIndex={selected ? 0 : -1}
            onClick={() => onChange(item.value)}
            onKeyDown={(e) => {
              const idx = items.findIndex((i) => i.value === value);
              const dir = document.dir === "rtl" ? -1 : 1;
              if (e.key === "ArrowRight") onChange(items[(idx + dir + items.length) % items.length]!.value);
              if (e.key === "ArrowLeft") onChange(items[(idx - dir + items.length) % items.length]!.value);
            }}
            className={cn(
              "-mb-px flex h-10 items-center gap-2 border-b-2 px-3 font-display text-sm transition-colors",
              selected ? "border-ink font-semibold text-ink" : "border-transparent text-slate hover:text-ink",
            )}
          >
            {item.label}
            {item.count !== undefined ? <span className="rounded-md bg-surface-muted px-1.5 text-xs text-slate">{item.count}</span> : null}
          </button>
        );
      })}
    </div>
  );
}
