"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useTranslations } from "next-intl";
import { cn } from "@/lib/cn";

const NAV = [
  { href: "/dashboard", key: "dashboard" },
  { href: "/studio", key: "studio" },
  { href: "/topics", key: "topics" },
  { href: "/library", key: "library" },
  { href: "/people", key: "people" },
  { href: "/reports", key: "reports" },
  { href: "/settings", key: "settings" },
  { href: "/audit", key: "audit" },
] as const;

export function Sidebar({ tenantName }: { tenantName: string }) {
  const pathname = usePathname();
  const t = useTranslations("nav");
  return (
    <aside className="hidden w-56 shrink-0 flex-col border-e border-line bg-surface md:flex">
      <div className="flex h-14 items-center gap-2 border-b border-line px-4">
        <span aria-hidden className="size-6 rounded-md bg-ink" />
        <span className="font-display text-base font-semibold text-ink">Reelearn</span>
      </div>
      <nav aria-label={t("primary")} className="flex flex-1 flex-col gap-0.5 p-2">
        {NAV.map((item) => {
          const active = pathname === item.href || pathname.startsWith(`${item.href}/`);
          return (
            <Link
              key={item.href}
              href={item.href}
              aria-current={active ? "page" : undefined}
              className={cn(
                "flex h-9 items-center rounded-[10px] px-3 font-display text-sm transition-colors",
                active ? "bg-ink-soft font-semibold text-ink" : "text-slate hover:bg-surface-muted hover:text-ink",
              )}
            >
              {t(item.key)}
            </Link>
          );
        })}
      </nav>
      <div className="border-t border-line px-4 py-3 text-xs text-slate">
        <p className="truncate font-medium text-ink">{tenantName}</p>
      </div>
    </aside>
  );
}
