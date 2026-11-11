import type { Metadata } from "next";
import { PlatformView } from "@/features/platform/platform-view";

export const metadata: Metadata = { title: "Platform" };

export default function PlatformPage() {
  return <PlatformView />;
}
