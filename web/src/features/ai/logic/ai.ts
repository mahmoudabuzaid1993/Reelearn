import type { AiPlan, ProviderAccess, ReelQuality } from "@/lib/api/types";

/** The AI providers a company routes work to, in display order. */
export const AI_PROVIDERS = ["ai_muse", "ai_claude", "ai_typesafe"] as const;
/** Media and messaging providers that also resolve per company. */
export const MEDIA_PROVIDERS = ["ai_higgsfield", "voice", "asr", "whatsapp"] as const;
export const ALL_PROVIDERS = [...AI_PROVIDERS, ...MEDIA_PROVIDERS] as const;
export type ProviderKind = (typeof ALL_PROVIDERS)[number];

/** What a company admin is told about one provider. */
export type AccessState = "company" | "platform" | "not_in_plan" | "budget_reached" | "not_configured";

export function accessState(p: ProviderAccess | undefined): AccessState {
  if (!p) return "not_configured";
  if (p.using === "company" || p.using === "platform") return p.using;
  return p.unavailable ?? "not_configured";
}

export function accessTone(state: AccessState): "mint" | "ink" | "neutral" | "coral" {
  switch (state) {
    case "company":
    case "platform":
      return "mint";
    case "budget_reached":
    case "not_in_plan":
      return "ink";
    default:
      return "neutral";
  }
}

/** Share of the monthly budget used, 0..1, or null without a budget. */
export function budgetShare(spent: number, budget: number | null | undefined): number | null {
  if (budget === null || budget === undefined) return null;
  if (budget <= 0) return 1;
  return Math.min(1, Math.max(0, spent / budget));
}

export function formatUsd(value: number | null | undefined, locale: string): string {
  if (value === null || value === undefined) return "—";
  const abs = Math.abs(value);
  const digits = abs === 0 || abs >= 1 ? 2 : abs >= 0.01 ? 4 : 6;
  return new Intl.NumberFormat(locale === "ar" ? "ar-SA" : "en-US", { style: "currency", currency: "USD", minimumFractionDigits: digits, maximumFractionDigits: digits }).format(value);
}

/** `YYYY-MM` for the given date's month, offset by `delta` months. */
export function monthKey(date: Date, delta = 0): string {
  const d = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + delta, 1));
  return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, "0")}`;
}

export interface PlanDraft {
  platform_keys: boolean;
  budget: string;
  markup: string;
  allowed: string[];
}

export function draftFrom(plan: AiPlan | undefined): PlanDraft {
  return {
    platform_keys: plan?.platform_keys ?? true,
    budget: plan?.monthly_budget_usd === null || plan?.monthly_budget_usd === undefined ? "" : String(plan.monthly_budget_usd),
    markup: String(plan?.markup_pct ?? 0),
    allowed: plan?.allowed ?? [],
  };
}

export type PlanError = "budget" | "markup";

/** Validate the plan form; an empty budget means unlimited. */
export function planFromDraft(d: PlanDraft): { plan: AiPlan } | { error: PlanError } {
  const budget = d.budget.trim() === "" ? null : Number(d.budget);
  if (budget !== null && (!Number.isFinite(budget) || budget < 0)) return { error: "budget" };
  const markup = Number(d.markup || "0");
  if (!Number.isFinite(markup) || markup < 0 || markup > 500) return { error: "markup" };
  return { plan: { platform_keys: d.platform_keys, monthly_budget_usd: budget, markup_pct: markup, allowed: d.allowed } };
}

export function qualityTone(q: ReelQuality | null | undefined): "mint" | "ink" | "neutral" {
  if (!q || q.status !== "checked") return "neutral";
  return q.needs_attention ? "ink" : "mint";
}

/** Settings fields per provider: credentials first, then options and (optional) prices. */
export const PROVIDER_FIELDS: Record<ProviderKind, { name: string; secret?: boolean; optional?: boolean }[]> = {
  ai_muse: [{ name: "api_key", secret: true }, { name: "model", optional: true }, { name: "base_url", optional: true }],
  ai_claude: [{ name: "api_key", secret: true }, { name: "model", optional: true }],
  ai_typesafe: [{ name: "api_key", secret: true }, { name: "model", optional: true }, { name: "base_url", optional: true }],
  ai_higgsfield: [{ name: "api_key", secret: true }, { name: "base_url" }, { name: "provider", optional: true }],
  voice: [{ name: "api_key", secret: true }, { name: "base_url" }, { name: "default_voice", optional: true }],
  asr: [{ name: "api_key", secret: true }, { name: "base_url" }],
  whatsapp: [{ name: "phone_number_id" }, { name: "access_token", secret: true }],
};

export const PRICE_FIELDS = ["price_input_per_mtok", "price_output_per_mtok", "price_per_call"] as const;

/** Only send fields the person filled in; an emptied price is removed (null). */
export function configPatch(values: Record<string, string>, touched: Set<string>): Record<string, string | number | null> {
  const out: Record<string, string | number | null> = {};
  for (const key of touched) {
    const raw = (values[key] ?? "").trim();
    if ((PRICE_FIELDS as readonly string[]).includes(key)) {
      out[key] = raw === "" ? null : Number(raw);
    } else if (raw !== "") {
      out[key] = raw;
    }
  }
  return out;
}
