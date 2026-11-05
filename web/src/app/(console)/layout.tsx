import type { ReactNode } from "react";
import { AuthProvider } from "@/lib/auth/provider";
import { ConsoleShell } from "@/components/layout/console-shell";

export default function ConsoleLayout({ children }: { children: ReactNode }) {
  return (
    <AuthProvider>
      <ConsoleShell>{children}</ConsoleShell>
    </AuthProvider>
  );
}
