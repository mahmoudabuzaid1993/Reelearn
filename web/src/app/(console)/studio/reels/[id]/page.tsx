import type { Metadata } from "next";
import { ReelEditor } from "@/features/studio/reel-editor";

export const metadata: Metadata = { title: "Reel editor" };

export default async function ReelPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return <ReelEditor reelId={id} />;
}
