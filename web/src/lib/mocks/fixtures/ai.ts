import type { AiOverview, AiPlan, AiRouting, AiUsage, PlatformIntegration, ProviderAccess, ReelQuality, TenantAi } from "@/lib/api/types";
import { IDS } from "./ids";

/** Mirrors `reelearn_domain::ai` defaults and the demo seed in `api/crates/api/src/seed.rs`. */
export interface MockAi {
  routing: AiRouting;
  plan: AiPlan;
  /** Provider kinds the company has its own key for. */
  companyKeys: string[];
  /** The platform's config per kind (secrets kept as "••••"). */
  platform: Record<string, Record<string, unknown>>;
  /** Billed platform-key spend this month before markup. */
  platformCost: number;
}

export const PROVIDERS = ["ai_claude", "ai_muse", "ai_typesafe", "ai_higgsfield", "voice", "asr", "whatsapp"] as const;

export function freshAi(): MockAi {
  return {
    routing: { writer: "auto", quality_check: true, review_threshold: 70, role_matching: true, tag_check: true },
    plan: { platform_keys: true, monthly_budget_usd: 150, markup_pct: 20, allowed: [] },
    companyKeys: ["ai_claude"],
    platform: {
      ai_muse: { api_key: "••••", model: "muse-spark-1.3", price_input_per_mtok: 1, price_output_per_mtok: 4, price_per_call: 0.02 },
      ai_typesafe: { api_key: "••••", model: "jev-latest" },
    },
    platformCost: 4.86,
  };
}

const MODELS: Record<string, string> = { ai_claude: "claude-sonnet-5-5", ai_muse: "muse-spark-1.3", ai_typesafe: "jev-latest" };

function hasKey(kind: string, cfg: Record<string, unknown> | undefined) {
  if (!cfg) return false;
  if (kind === "whatsapp") return Boolean(cfg.access_token && cfg.phone_number_id);
  const needsUrl = kind === "ai_higgsfield" || kind === "voice" || kind === "asr";
  return Boolean(cfg.api_key) && (!needsUrl || Boolean(cfg.base_url));
}

export function overview(ai: MockAi): AiOverview {
  const billable = ai.platformCost * (1 + (ai.plan.markup_pct ?? 0) / 100);
  const providers: ProviderAccess[] = PROVIDERS.map((kind) => {
    const company = ai.companyKeys.includes(kind);
    const platform = hasKey(kind, ai.platform[kind]);
    const allowed = (ai.plan.allowed ?? []).length === 0 || (ai.plan.allowed ?? []).includes(kind);
    const budgetLeft = ai.plan.monthly_budget_usd === null || ai.plan.monthly_budget_usd === undefined || billable < ai.plan.monthly_budget_usd;
    const model = (ai.platform[kind]?.model as string | undefined) ?? MODELS[kind] ?? null;
    if (company) return { kind, company_key: true, platform_key: platform, using: "company", unavailable: null, message: null, model: MODELS[kind] ?? null };
    if (!platform) return { kind, company_key: false, platform_key: false, using: null, unavailable: "not_configured", message: "no API key is set for this provider", model: null };
    if (!ai.plan.platform_keys || !allowed) return { kind, company_key: false, platform_key: true, using: null, unavailable: "not_in_plan", message: null, model: null };
    if (!budgetLeft) return { kind, company_key: false, platform_key: true, using: null, unavailable: "budget_reached", message: null, model: null };
    return { kind, company_key: false, platform_key: true, using: "platform", unavailable: null, message: null, model };
  });
  return {
    routing: ai.routing,
    plan: { platform_keys: ai.plan.platform_keys ?? true, allowed: ai.plan.allowed ?? [], monthly_budget_usd: ai.plan.monthly_budget_usd ?? null, spent_this_month_usd: Math.round(billable * 1e6) / 1e6 },
    providers,
  };
}

export function usage(month: string, ai: MockAi): AiUsage {
  const [y, m] = month.split("-").map(Number) as [number, number];
  const from = new Date(Date.UTC(y, m - 1, 1));
  const to = new Date(Date.UTC(y, m, 1));
  const now = new Date();
  const lastDay = now >= to ? new Date(to.getTime() - 86_400_000).getUTCDate() : now.getUTCDate();
  const days = Array.from({ length: Math.max(0, lastDay) }, (_, i) => {
    const d = new Date(Date.UTC(y, m - 1, i + 1));
    const calls = 6 + ((i * 7) % 5);
    return { day: d.toISOString().slice(0, 10), calls, cost_usd: Math.round(calls * 0.061 * 1e4) / 1e4 };
  });
  const lines: AiUsage["lines"] = [
    { provider: "ai_muse", capability: "write", key_source: "platform", calls: 14, failed: 0, input_tokens: 669_900, output_tokens: 91_000, units: 14, cost_usd: 1.0339 },
    { provider: "ai_muse", capability: "render", key_source: "platform", calls: 14, failed: 1, input_tokens: 0, output_tokens: 0, units: 42, cost_usd: 0.84 },
    { provider: "ai_typesafe", capability: "judge", key_source: "platform", calls: 14, failed: 0, input_tokens: 963_200, output_tokens: 4_200, units: 14, cost_usd: 0.040454 },
    { provider: "ai_claude", capability: "write", key_source: "company", calls: 4, failed: 0, input_tokens: 72_000, output_tokens: 12_000, units: 4, cost_usd: 0.396 },
  ];
  const platform = lines.filter((l) => l.key_source === "platform").reduce((s, l) => s + (l.cost_usd ?? 0), 0);
  return {
    from: from.toISOString(),
    to: to.toISOString(),
    lines: days.length ? lines : [],
    days,
    platform_billable_usd: days.length ? platform * (1 + (ai.plan.markup_pct ?? 0) / 100) : 0,
    company_cost_usd: days.length ? 0.396 : 0,
  };
}

export function platformIntegrations(ai: MockAi): PlatformIntegration[] {
  return PROVIDERS.map((kind) => {
    const cfg = ai.platform[kind];
    const usable = hasKey(kind, cfg);
    return { kind, status: usable ? "connected" : "disconnected", source: usable ? "console" : null, last_error: null, public_config: cfg ?? {} };
  });
}

export function tenantsAi(ai: MockAi, tenantName: string): TenantAi[] {
  const billable = ai.platformCost * (1 + (ai.plan.markup_pct ?? 0) / 100);
  return [
    {
      id: IDS.tenant,
      slug: "alwadi",
      name: tenantName,
      created_at: "2026-10-01T08:00:00Z",
      plan: ai.plan,
      calls: 46,
      company_calls: 4,
      platform_calls: 42,
      platform_cost_usd: ai.platformCost,
      billable_usd: billable,
      company_cost_usd: 0.396,
      unpriced_calls: 0,
      budget_used: ai.plan.monthly_budget_usd ? Math.min(1, billable / ai.plan.monthly_budget_usd) : null,
    },
    {
      id: "0196f0a0-0000-7000-8000-00000000b001",
      slug: "nakheel-clinics",
      name: "Nakheel Clinics",
      created_at: "2026-10-20T08:00:00Z",
      plan: { platform_keys: false, monthly_budget_usd: null, markup_pct: 0, allowed: [] },
      calls: 120,
      company_calls: 120,
      platform_calls: 0,
      platform_cost_usd: 0,
      billable_usd: 0,
      company_cost_usd: 3.12,
      unpriced_calls: 18,
      budget_used: null,
    },
  ];
}

export const QUALITY: Record<"good" | "flagged", ReelQuality> = {
  good: { status: "checked", score: 88, needs_attention: false, flags: [], judge: "typesafe-jev", message: null, checked_at: "2026-11-09T09:12:00Z" },
  flagged: { status: "checked", score: 52, needs_attention: true, flags: ["not_grounded", "answer_key_wrong", "weak_hook"], judge: "typesafe-jev", message: null, checked_at: "2026-11-09T09:14:00Z" },
};
