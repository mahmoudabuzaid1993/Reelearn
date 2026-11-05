import type { Metadata } from "next";
import { TopicsStep } from "@/features/onboarding/topics-step";

export const metadata: Metadata = { title: "Topics and people" };

export default function OnboardingTopicsPage() {
  return <TopicsStep />;
}
