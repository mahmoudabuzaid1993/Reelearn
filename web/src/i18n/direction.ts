import { type Locale } from "./locales";

export type Direction = "ltr" | "rtl";

const RTL_LANGUAGES = new Set(["ar", "ur", "fa", "he"]);

/** Text direction for a UI locale or a learner language code. */
export function directionFor(locale: string): Direction {
  const base = locale.toLowerCase().split(/[-_]/)[0] ?? locale;
  return RTL_LANGUAGES.has(base) ? "rtl" : "ltr";
}

export function isRtl(locale: Locale | string): boolean {
  return directionFor(locale) === "rtl";
}

/** The locale to switch to from the current one (two-locale console). */
export function otherLocale(locale: Locale): Locale {
  return locale === "ar" ? "en" : "ar";
}
