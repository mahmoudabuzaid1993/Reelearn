import { describe, expect, it } from "vitest";
import { buildQuote, creditsAfter, estimatedMinutes, isAffordable, quoteLines, shortfall, totalCredits } from "../quote";

describe("quote arithmetic", () => {
  it("prices 3 avatar reels in 2 languages at 63 credits", () => {
    const lines = quoteLines({ reel_count: 3, languages: ["en", "ar"], style: "ai_avatar" });
    expect(lines).toEqual([
      { step: "script", credits: 6 },
      { step: "question", credits: 3 },
      { step: "render", credits: 36 },
      { step: "dub", credits: 12 },
      { step: "captions", credits: 6 },
    ]);
    expect(totalCredits(lines)).toBe(63);
  });

  it("drops the dubbing line for a single language and uses the style rate", () => {
    const lines = quoteLines({ reel_count: 2, languages: ["ar"], style: "broll" });
    expect(lines.find((l) => l.step === "dub")).toBeUndefined();
    expect(lines.find((l) => l.step === "render")?.credits).toBe(12);
    expect(totalCredits(lines)).toBe(2 * (2 + 1 + 6 + 1));
  });

  it("counts duplicate languages once and clamps reel_count at zero", () => {
    expect(totalCredits(quoteLines({ reel_count: 1, languages: ["ar", "ar"], style: "motion" }))).toBe(2 + 1 + 10 + 1);
    expect(totalCredits(quoteLines({ reel_count: -3, languages: ["ar"], style: "motion" }))).toBe(0);
  });

  it("reports affordability and shortfall against available credits", () => {
    const quote = buildQuote({ reel_count: 3, languages: ["en", "ar"], style: "ai_avatar" }, 1200);
    expect(quote.cost_credits).toBe(63);
    expect(creditsAfter(quote)).toBe(1137);
    expect(isAffordable(quote)).toBe(true);
    expect(shortfall(quote)).toBe(0);

    const poor = buildQuote({ reel_count: 12, languages: ["en", "ar", "ur"], style: "ai_avatar" }, 100);
    expect(isAffordable(poor)).toBe(false);
    expect(shortfall(poor)).toBe((poor.cost_credits ?? 0) - 100);
  });

  it("estimates minutes with a floor of one", () => {
    expect(estimatedMinutes({ reel_count: 1, languages: ["ar"], style: "broll" })).toBe(2);
    expect(estimatedMinutes({ reel_count: 0, languages: ["ar"], style: "broll" })).toBe(1);
  });
});
