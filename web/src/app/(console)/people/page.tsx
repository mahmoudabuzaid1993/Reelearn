import type { Metadata } from "next";
import { Suspense } from "react";
import { PeopleView } from "@/features/people/people-view";

export const metadata: Metadata = { title: "People" };

export default function PeoplePage() {
  return (
    <Suspense>
      <PeopleView />
    </Suspense>
  );
}
