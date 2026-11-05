import type { Metadata } from "next";
import { TopicsView } from "@/features/topics/topics-view";

export const metadata: Metadata = { title: "Topics" };

export default function TopicsPage() {
  return <TopicsView />;
}
