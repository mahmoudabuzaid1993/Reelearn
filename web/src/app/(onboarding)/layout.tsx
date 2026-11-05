import type { ReactNode } from "react";
import { AuthProvider } from "@/lib/auth/provider";
import { OnboardingShell } from "@/features/onboarding/onboarding-shell";

export default function OnboardingLayout({ children }: { children: ReactNode }) {
  return (
    <AuthProvider>
      <OnboardingShell>{children}</OnboardingShell>
    </AuthProvider>
  );
}
