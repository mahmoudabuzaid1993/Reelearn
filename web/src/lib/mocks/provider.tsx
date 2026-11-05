"use client";

import { useEffect, useState, type ReactNode } from "react";
import { isMockMode } from "@/lib/auth/config";

/** In mock mode, hold rendering until the mock layer is ready so no request slips past it. */
export function MockProvider({ children }: { children: ReactNode }) {
  const [ready, setReady] = useState(() => !isMockMode());

  useEffect(() => {
    if (ready) return;
    let cancelled = false;
    (async () => {
      try {
        const { worker } = await import("./browser");
        await worker.start({ onUnhandledRequest: "bypass", quiet: true });
      } catch {
        const { installFetchFallback } = await import("./fetch-fallback");
        installFetchFallback();
      }
      if (!cancelled) setReady(true);
    })();
    return () => {
      cancelled = true;
    };
  }, [ready]);

  if (!ready) return <div aria-busy="true" className="min-h-screen bg-paper" />;
  return <>{children}</>;
}
