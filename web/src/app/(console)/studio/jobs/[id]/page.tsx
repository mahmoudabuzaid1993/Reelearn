import type { Metadata } from "next";
import { JobProgress } from "@/features/studio/job-progress";

export const metadata: Metadata = { title: "Generating" };

export default async function JobPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return <JobProgress jobId={id} />;
}
