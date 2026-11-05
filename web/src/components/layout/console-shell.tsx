"use client";

import { useEffect, type ReactNode } from "react";
import { useRouter } from "next/navigation";
import { useAuth } from "@/lib/auth/provider";
import { Sidebar } from "./sidebar";
import { Topbar } from "./topbar";
import { SkeletonRows } from "@/components/ui/skeleton";

/** Waits for the in-memory access token; redirects to /signin if the refresh cookie is dead. */
export function ConsoleShell({ children }: { children: ReactNode }) {
  const { status, user } = useAuth();
  const router = useRouter();

  useEffect(() => {
    if (status === "anonymous") router.replace("/signin");
  }, [status, router]);

  if (status !== "authenticated" || !user) {
    return (
      <div className="mx-auto max-w-5xl p-8">
        <SkeletonRows rows={6} />
      </div>
    );
  }

  return (
    <div className="flex min-h-screen">
      <Sidebar tenantName={user.tenant.name} />
      <div className="flex min-w-0 flex-1 flex-col">
        <Topbar />
        <main id="main" className="flex-1 px-4 py-6 md:px-8">
          <div className="mx-auto max-w-6xl">{children}</div>
        </main>
      </div>
    </div>
  );
}
