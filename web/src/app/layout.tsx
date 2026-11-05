import type { Metadata } from "next";
import { IBM_Plex_Sans_Arabic, Readex_Pro } from "next/font/google";
import { NextIntlClientProvider } from "next-intl";
import { getLocale, getMessages } from "next-intl/server";
import { directionFor } from "@/i18n/direction";
import { Providers } from "./providers";
import "./globals.css";

const readex = Readex_Pro({ subsets: ["latin", "arabic"], weight: ["400", "500", "600", "700"], variable: "--font-readex", display: "swap" });
const plexArabic = IBM_Plex_Sans_Arabic({ subsets: ["latin", "arabic"], weight: ["400", "500", "600"], variable: "--font-plex-arabic", display: "swap" });

export const metadata: Metadata = {
  title: { default: "Reelearn", template: "%s · Reelearn" },
  description: "Reelearn admin console and AI Reel Studio",
};

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  const locale = await getLocale();
  const messages = await getMessages();
  return (
    <html lang={locale} dir={directionFor(locale)} className={`${readex.variable} ${plexArabic.variable}`}>
      <body className="min-h-screen bg-paper text-ink">
        <NextIntlClientProvider locale={locale} messages={messages}>
          <Providers>{children}</Providers>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}
