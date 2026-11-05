import type { Metadata } from "next";
import { StudioIndex } from "@/features/studio/studio-index";

export const metadata: Metadata = { title: "Studio" };

export default function StudioPage() {
  return <StudioIndex />;
}
