import type { Metadata } from "next";
import { EngineView } from "@/features/engine/engine-view";

export const metadata: Metadata = { title: "Learning engine" };

export default function EnginePage() {
  return <EngineView />;
}
