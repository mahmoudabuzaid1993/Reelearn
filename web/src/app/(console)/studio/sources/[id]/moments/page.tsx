import type { Metadata } from "next";
import { Moments } from "@/features/studio/moments";

export const metadata: Metadata = { title: "Moments" };

export default async function MomentsPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return <Moments sourceId={id} />;
}
