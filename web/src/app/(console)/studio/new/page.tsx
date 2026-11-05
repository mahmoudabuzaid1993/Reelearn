import type { Metadata } from "next";
import { NewSource } from "@/features/studio/new-source";

export const metadata: Metadata = { title: "New source" };

export default function NewSourcePage() {
  return <NewSource />;
}
