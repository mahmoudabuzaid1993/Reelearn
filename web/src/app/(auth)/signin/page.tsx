import type { Metadata } from "next";
import { Suspense } from "react";
import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { SignInForm } from "@/features/auth/signin-form";

export const metadata: Metadata = { title: "Sign in" };

export default async function SignInPage({ searchParams }: { searchParams: Promise<{ challenge?: string; slug?: string }> }) {
  const t = await getTranslations("auth");
  const sp = await searchParams;
  return (
    <div className="flex flex-col gap-4">
      <Suspense>
        <SignInForm initialSlug={sp.slug ?? "alwadi"} initialChallengeId={sp.challenge ?? null} />
      </Suspense>
      <p className="text-center text-sm text-slate">
        {t("noWorkspace")}{" "}
        <Link href="/signup" className="font-medium text-ink underline underline-offset-2">
          {t("createOne")}
        </Link>
      </p>
    </div>
  );
}
