import type { Metadata } from "next";
import { DeliveryStep } from "@/features/onboarding/delivery-step";

export const metadata: Metadata = { title: "Delivery and schedule" };

export default function OnboardingDeliveryPage() {
  return <DeliveryStep />;
}
