"use client";

import Link from "next/link";
import { useTranslations } from "next-intl";
import { useAuth } from "@/lib/auth/provider";
import { Button } from "@/components/ui/button";
import { LocaleSwitch } from "./locale-switch";

const ROLE_LABELS: Record<string, string> = { admin: "admin", manager: "manager", sme: "sme", learner: "learner" };

export function Topbar() {
  const { user, signOut } = useAuth();
  const t = useTranslations("common");
  return (
    <header className="flex h-14 items-center justify-between border-b border-line bg-surface px-4 md:px-6">
      <nav aria-label={t("mobileNav")} className="flex items-center gap-3 md:hidden">
        <Link href="/dashboard" className="font-display font-semibold text-ink">
          Reelearn
        </Link>
      </nav>
      <div className="hidden md:block" />
      <div className="flex items-center gap-2">
        <LocaleSwitch />
        {user ? (
          <span className="hidden text-[13px] text-slate sm:inline">
            {user.full_name} · {t(`roles.${ROLE_LABELS[user.role] ?? "learner"}`)}
          </span>
        ) : null}
        <Button size="sm" variant="quiet" onClick={() => void signOut()}>
          {t("signOut")}
        </Button>
      </div>
    </header>
  );
}
