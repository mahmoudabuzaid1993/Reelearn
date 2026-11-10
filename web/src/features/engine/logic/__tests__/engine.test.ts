import { describe, expect, it } from "vitest";
import { enginePresets } from "@/lib/mocks/fixtures/engine";
import { changedFields, clampTo, labelFor, NUMBER_RANGES, patchFor, setNumber, setWeight, WEIGHT_RANGES } from "../engine";

const balanced = enginePresets.balanced!;

describe("engine settings logic", () => {
  it("recognises each preset and labels an edit as custom", () => {
    for (const [name, preset] of Object.entries(enginePresets)) {
      expect(labelFor(preset, enginePresets)).toBe(name);
    }
    expect(labelFor(setNumber(balanced, "daily_minutes_cap", 25), enginePresets)).toBe("custom");
  });

  it("clamps to the API's ranges and steps", () => {
    expect(clampTo(0.93, NUMBER_RANGES.exploration_rate)).toBe(0.5);
    expect(clampTo(7, NUMBER_RANGES.daily_minutes_cap)).toBe(5);
    expect(clampTo(Number.NaN, NUMBER_RANGES.batch_size)).toBe(3);
    // The two penalties cannot go below 1: people's own signals always count.
    expect(setWeight(balanced, "not_relevant_penalty", 0).weights.not_relevant_penalty).toBe(WEIGHT_RANGES.not_relevant_penalty.min);
  });

  it("sends only what changed, with weights as a partial object", () => {
    const draft = setWeight(setNumber(balanced, "session_minutes_cap", 20), "share", 2.5);
    expect(changedFields(balanced, draft)).toEqual(["weights.share", "session_minutes_cap"]);
    expect(patchFor(balanced, draft)).toEqual({ session_minutes_cap: 20, weights: { share: 2.5 } });
    expect(patchFor(balanced, balanced)).toEqual({});
  });
});
