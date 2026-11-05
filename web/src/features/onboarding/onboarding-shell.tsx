"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useTranslations } from "next-intl";
import { useEffect, type ReactNode } from "react";
import { useAuth } from "@/lib/auth/provider";
import { LocaleSwitch } from "@/components/layout/locale-switch";
import { SkeletonRows } from "@/components/ui/skeleton";
import { cn } from "@/lib/cn";

const STEPS = [
  { href: "/signup", key: "workspace" },
  { href: "/onboarding/topics", key: "topics" },
  { href: "/onboarding/delivery", key: "delivery" },
  { href: "/dashboard", key: "dashboard" },
] as const;

export function OnboardingShell({ children }: { children: ReactNode }) {
  const { status } = useAuth();
  const router = useRouter();
  const pathname = usePathname();
  const t = useTranslations("onboarding");
  useEffect(() => {
    if (status === "anonymous") router.replace(`/signin?next=${encodeURIComponent(pathname)}`);
  }, [status, router, pathname]);
  const current = STEPS.findIndex((s) => pathname.startsWith(s.href));
  return (
    <div className="flex min-h-screen flex-col">
      <header className="flex h-14 items-center justify-between border-b border-line bg-surface px-6">
        <span className="font-display text-base font-semibold text-ink">Reelearn</span>
        <LocaleSwitch />
      </header>
      <main className="mx-auto w-full max-w-4xl flex-1 px-4 py-8">
        <ol className="mb-8 flex flex-wrap gap-2" aria-label={t("steps")}>
          {STEPS.map((s, i) => (
            <li key={s.key} className="flex items-center gap-2 text-sm">
              <Link
                href={s.href}
                aria-current={i === current ? "step" : undefined}
                className={cn("flex items-center gap-2 rounded-full border px-3 py-1", i === current ? "border-ink bg-ink text-white" : i < current ? "border-mint bg-mint-soft text-ink" : "border-line text-slate")}
              >
                <span className="text-xs">{i + 1}</span>
                {t(`stepNames.${s.key}`)}
              </Link>
              {i < STEPS.length - 1 ? <span aria-hidden className="text-line">—</span> : null}
            </li>
          ))}
        </ol>
        {status === "authenticated" ? children : <SkeletonRows rows={6} />}
      </main>
    </div>
  );
}
