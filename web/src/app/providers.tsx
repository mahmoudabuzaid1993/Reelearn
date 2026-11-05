"use client";

import type { ReactNode } from "react";
import { MockProvider } from "@/lib/mocks/provider";
import { QueryProvider } from "@/lib/query/provider";
import { ToastProvider } from "@/components/ui/toast";

export function Providers({ children }: { children: ReactNode }) {
  return (
    <MockProvider>
      <QueryProvider>
        <ToastProvider>{children}</ToastProvider>
      </QueryProvider>
    </MockProvider>
  );
}
