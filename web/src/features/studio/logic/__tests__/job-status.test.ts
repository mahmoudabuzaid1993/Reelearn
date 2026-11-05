import { describe, expect, it } from "vitest";
import { canOpenStoryboard, failedStep, isTerminal, progressFromSteps } from "../job-status";

describe("job status helpers", () => {
  it("finds the failed step and terminal states", () => {
    const steps = [
      { position: 1, step: "extract", status: "done" },
      { position: 2, step: "render", status: "failed" },
    ];
    expect(failedStep({ steps })?.step).toBe("render");
    expect(isTerminal("failed")).toBe(true);
    expect(isTerminal("running")).toBe(false);
  });

  it("only opens the storyboard when done with reels", () => {
    expect(canOpenStoryboard({ status: "done", reel_ids: ["a"] })).toBe(true);
    expect(canOpenStoryboard({ status: "done", reel_ids: [] })).toBe(false);
    expect(canOpenStoryboard({ status: "running", reel_ids: ["a"] })).toBe(false);
  });

  it("computes progress from steps with half credit for a running step", () => {
    expect(progressFromSteps([])).toBe(0);
    expect(
      progressFromSteps([
        { position: 1, step: "a", status: "done" },
        { position: 2, step: "b", status: "running" },
        { position: 3, step: "c", status: "pending" },
        { position: 4, step: "d", status: "pending" },
      ]),
    ).toBe(38);
  });
});
