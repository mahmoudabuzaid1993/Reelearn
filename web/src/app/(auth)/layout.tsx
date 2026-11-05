import type { ReactNode } from "react";
import { LocaleSwitch } from "@/components/layout/locale-switch";

export default function AuthLayout({ children }: { children: ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col">
      <header className="flex h-14 items-center justify-between px-6">
        <span className="font-display text-base font-semibold text-ink">Reelearn</span>
        <LocaleSwitch />
      </header>
      <main className="flex flex-1 items-start justify-center px-4 py-10">
        <div className="w-full max-w-md">{children}</div>
      </main>
    </div>
  );
}
