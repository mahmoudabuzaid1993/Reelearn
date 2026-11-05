import type { Metadata } from "next";
import { Publish } from "@/features/studio/publish";

export const metadata: Metadata = { title: "Approve and publish" };

export default async function PublishPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return <Publish reelId={id} />;
}
