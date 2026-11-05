import type { Metadata } from "next";
import { Suspense } from "react";
import { GenerationSetup } from "@/features/studio/generation-setup";

export const metadata: Metadata = { title: "Generation setup" };

export default function GenerationSetupPage() {
  return (
    <Suspense>
      <GenerationSetup />
    </Suspense>
  );
}
