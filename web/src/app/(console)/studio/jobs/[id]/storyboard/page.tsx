import type { Metadata } from "next";
import { Storyboard } from "@/features/studio/storyboard";

export const metadata: Metadata = { title: "Storyboard" };

export default async function StoryboardPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return <Storyboard jobId={id} />;
}
