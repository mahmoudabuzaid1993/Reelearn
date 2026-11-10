import type { EngineSettings, EngineWeights, Intensity } from "@/lib/api/types";

export const PRESETS: Exclude<Intensity, "custom">[] = ["gentle", "balanced", "intense", "maximum"];

export type WeightKey = keyof EngineWeights;
export type NumberKey =
  | "exploration_rate"
  | "interest_half_life_days"
  | "max_same_topic_in_row"
  | "review_every"
  | "target_correct_rate"
  | "daily_minutes_cap"
  | "session_minutes_cap"
  | "break_reminder_minutes"
  | "max_nudges_per_day"
  | "reengage_after_hours"
  | "batch_size";
export type ToggleKey =
  | "open_catalog"
  | "social_proof"
  | "peer_sharing"
  | "continuous_enabled"
  | "practice_outside_work_hours"
  | "autoplay_next";

export interface Range {
  min: number;
  max: number;
  step: number;
  /** Shown when the value is 0 (for caps: "no cap"). */
  zeroLabel?: "noCap" | "never" | "off";
  /** Values are fractions shown as percentages. */
  percent?: boolean;
}

/** Mirrors `EngineSettings::validate` in the Rust domain crate. */
export const WEIGHT_RANGES: Record<WeightKey, Range> = {
  completion: { min: 0, max: 5, step: 0.1 },
  learning: { min: 0, max: 5, step: 0.1 },
  save: { min: 0, max: 5, step: 0.1 },
  share: { min: 0, max: 5, step: 0.1 },
  interest: { min: 0, max: 5, step: 0.1 },
  urgency: { min: 0, max: 5, step: 0.1 },
  freshness: { min: 0, max: 5, step: 0.1 },
  social: { min: 0, max: 5, step: 0.1 },
  skip_penalty: { min: 1, max: 5, step: 0.1 },
  not_relevant_penalty: { min: 1, max: 5, step: 0.1 },
  repeat_penalty: { min: 0, max: 5, step: 0.1 },
};

export const NUMBER_RANGES: Record<NumberKey, Range> = {
  exploration_rate: { min: 0, max: 0.5, step: 0.01, percent: true },
  interest_half_life_days: { min: 1, max: 90, step: 1 },
  max_same_topic_in_row: { min: 1, max: 5, step: 1 },
  review_every: { min: 0, max: 10, step: 1, zeroLabel: "never" },
  target_correct_rate: { min: 0.5, max: 0.95, step: 0.01, percent: true },
  daily_minutes_cap: { min: 0, max: 240, step: 5, zeroLabel: "noCap" },
  session_minutes_cap: { min: 0, max: 240, step: 5, zeroLabel: "noCap" },
  break_reminder_minutes: { min: 0, max: 120, step: 5, zeroLabel: "off" },
  max_nudges_per_day: { min: 0, max: 6, step: 1, zeroLabel: "off" },
  reengage_after_hours: { min: 0, max: 72, step: 1, zeroLabel: "never" },
  batch_size: { min: 3, max: 20, step: 1 },
};

export const GROUPS: { key: string; numbers: NumberKey[]; toggles: ToggleKey[] }[] = [
  { key: "practice", numbers: ["daily_minutes_cap", "session_minutes_cap", "break_reminder_minutes", "batch_size"], toggles: ["continuous_enabled", "practice_outside_work_hours", "autoplay_next"] },
  { key: "discovery", numbers: ["exploration_rate", "interest_half_life_days", "max_same_topic_in_row"], toggles: ["open_catalog", "social_proof", "peer_sharing"] },
  { key: "learning", numbers: ["review_every", "target_correct_rate"], toggles: [] },
  { key: "reminders", numbers: ["max_nudges_per_day", "reengage_after_hours"], toggles: [] },
];

export const POSITIVE_WEIGHTS: WeightKey[] = ["completion", "learning", "save", "share", "interest", "urgency", "freshness", "social"];
export const NEGATIVE_WEIGHTS: WeightKey[] = ["skip_penalty", "not_relevant_penalty", "repeat_penalty"];

function round(v: number): number {
  return Math.round(v * 1000) / 1000;
}

export function clampTo(value: number, r: Range): number {
  if (!Number.isFinite(value)) return r.min;
  const stepped = Math.round((value - r.min) / r.step) * r.step + r.min;
  return round(Math.min(r.max, Math.max(r.min, stepped)));
}

/** Field paths whose value differs between two settings (weights as `weights.<name>`). */
export function changedFields(a: EngineSettings, b: EngineSettings): string[] {
  const out: string[] = [];
  for (const key of Object.keys(a) as (keyof EngineSettings)[]) {
    if (key === "intensity") continue;
    if (key === "weights") {
      for (const w of Object.keys(a.weights) as WeightKey[]) {
        if (round(a.weights[w] ?? 0) !== round(b.weights[w] ?? 0)) out.push(`weights.${w}`);
      }
    } else if (a[key] !== b[key]) {
      out.push(key);
    }
  }
  return out;
}

/** The preset the values match, else "custom" (mirrors `matching_intensity`). */
export function labelFor(settings: EngineSettings, presets: Record<string, EngineSettings>): Intensity {
  for (const p of PRESETS) {
    const preset = presets[p];
    if (preset && changedFields(settings, preset).length === 0) return p;
  }
  return "custom";
}

export function setWeight(s: EngineSettings, key: WeightKey, value: number): EngineSettings {
  return { ...s, weights: { ...s.weights, [key]: clampTo(value, WEIGHT_RANGES[key]) } };
}

export function setNumber(s: EngineSettings, key: NumberKey, value: number): EngineSettings {
  return { ...s, [key]: clampTo(value, NUMBER_RANGES[key]) };
}

export function setToggle(s: EngineSettings, key: ToggleKey, value: boolean): EngineSettings {
  return { ...s, [key]: value };
}

/** Only the fields that changed, as a PUT body; weights go as a partial object. */
export function patchFor(saved: EngineSettings, draft: EngineSettings): Partial<EngineSettings> {
  const body: Record<string, unknown> = {};
  const weights: Record<string, number> = {};
  for (const path of changedFields(saved, draft)) {
    if (path.startsWith("weights.")) {
      const w = path.slice("weights.".length) as WeightKey;
      weights[w] = draft.weights[w] ?? 0;
    } else {
      body[path] = draft[path as keyof EngineSettings];
    }
  }
  if (Object.keys(weights).length > 0) body.weights = weights;
  return body as Partial<EngineSettings>;
}

/** Practice minutes a day, as a plain sentence key for the preset cards. */
export function capSummary(s: EngineSettings): { practice: boolean; dailyCap: number; reminders: number } {
  return { practice: s.continuous_enabled, dailyCap: s.daily_minutes_cap, reminders: s.max_nudges_per_day };
}
