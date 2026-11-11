import { describe, expect, it } from "vitest";
import { accessState, budgetShare, configPatch, draftFrom, formatUsd, monthKey, planFromDraft, qualityTone } from "../ai";

describe("accessState", () => {
  it("prefers what the API resolved", () => {
    expect(accessState({ kind: "ai_claude", company_key: true, platform_key: true, using: "company" })).toBe("company");
    expect(accessState({ kind: "ai_claude", company_key: false, platform_key: true, using: "platform" })).toBe("platform");
    expect(accessState({ kind: "ai_claude", company_key: false, platform_key: true, using: null, unavailable: "budget_reached" })).toBe("budget_reached");
    expect(accessState(undefined)).toBe("not_configured");
  });
});

describe("budget", () => {
  it("is a share of the budget, capped, or null when unlimited", () => {
    expect(budgetShare(30, 120)).toBe(0.25);
    expect(budgetShare(300, 120)).toBe(1);
    expect(budgetShare(3, null)).toBeNull();
    expect(budgetShare(0, 0)).toBe(1);
  });
});

describe("plan form", () => {
  it("round-trips and treats an empty budget as unlimited", () => {
    const d = draftFrom({ platform_keys: true, monthly_budget_usd: 150, markup_pct: 20, allowed: [] });
    expect(d.budget).toBe("150");
    expect(planFromDraft(d)).toEqual({ plan: { platform_keys: true, monthly_budget_usd: 150, markup_pct: 20, allowed: [] } });
    expect(planFromDraft({ ...d, budget: "" })).toEqual({ plan: { platform_keys: true, monthly_budget_usd: null, markup_pct: 20, allowed: [] } });
  });

  it("rejects negative budgets and out-of-range markups", () => {
    const d = draftFrom(undefined);
    expect(planFromDraft({ ...d, budget: "-1" })).toEqual({ error: "budget" });
    expect(planFromDraft({ ...d, markup: "900" })).toEqual({ error: "markup" });
  });
});

describe("config patch", () => {
  it("sends only touched fields, numbers for prices, null for cleared prices", () => {
    const touched = new Set(["api_key", "model", "price_input_per_mtok", "price_per_call"]);
    expect(configPatch({ api_key: " k ", model: "", price_input_per_mtok: "3", price_per_call: "" }, touched)).toEqual({ api_key: "k", price_input_per_mtok: 3, price_per_call: null });
  });
});

describe("formatting", () => {
  it("shows small amounts with more precision", () => {
    expect(formatUsd(0.000042, "en")).toBe("$0.000042");
    expect(formatUsd(0.25, "en")).toBe("$0.2500");
    expect(formatUsd(12.5, "en")).toBe("$12.50");
    expect(formatUsd(null, "en")).toBe("—");
  });

  it("builds month keys across a year end", () => {
    expect(monthKey(new Date(Date.UTC(2026, 11, 15)), 1)).toBe("2027-01");
    expect(monthKey(new Date(Date.UTC(2026, 0, 15)), -1)).toBe("2025-12");
  });

  it("tones the quality check", () => {
    expect(qualityTone({ status: "checked", needs_attention: false, flags: [] })).toBe("mint");
    expect(qualityTone({ status: "checked", needs_attention: true, flags: ["answer_key_wrong"] })).toBe("ink");
    expect(qualityTone({ status: "unavailable", needs_attention: false, flags: [] })).toBe("neutral");
  });
});
