import type { Metadata } from "next";
import Link from "next/link";
import { getTranslations } from "next-intl/server";
import { SignUpForm } from "@/features/auth/signup-form";

export const metadata: Metadata = { title: "Create workspace" };

export default async function SignUpPage() {
  const t = await getTranslations("signup");
  return (
    <div className="flex flex-col gap-4">
      <SignUpForm />
      <p className="text-center text-sm text-slate">
        {t("haveWorkspace")}{" "}
        <Link href="/signin" className="font-medium text-ink underline underline-offset-2">
          {t("signIn")}
        </Link>
      </p>
    </div>
  );
}
