import { describe, expect, it } from "vitest";
import type { Assignment } from "@/lib/api/types";
import { assignedGroupCount, isDirty, matrixReducer, toWriteList, type MatrixState } from "../matrix";

const topicId = "t1";
const assignments: Assignment[] = [
  { id: "a1", topic_id: topicId, group: { id: "g1", name: "Store 12" }, reels_per_day: 2, channel: "app", is_mandatory: false, starts_on: "2026-10-01", ends_on: null },
];
const defaults = { reels_per_day: 2, channel: "app" as const, is_mandatory: false, starts_on: "2026-10-30", ends_on: null };

describe("assignment matrix reducer", () => {
  it("hydrates from API assignments", () => {
    const state = matrixReducer({}, { type: "hydrate", topicId, assignments });
    expect(assignedGroupCount(state, topicId)).toBe(1);
    expect(state[topicId]?.g1?.channel).toBe("app");
    expect(isDirty(state, topicId, assignments)).toBe(false);
  });

  it("toggles a cell on and off", () => {
    let state: MatrixState = matrixReducer({}, { type: "hydrate", topicId, assignments });
    state = matrixReducer(state, { type: "toggle", topicId, groupId: "g2", defaults });
    expect(assignedGroupCount(state, topicId)).toBe(2);
    expect(isDirty(state, topicId, assignments)).toBe(true);
    state = matrixReducer(state, { type: "toggle", topicId, groupId: "g2", defaults });
    expect(assignedGroupCount(state, topicId)).toBe(1);
    expect(isDirty(state, topicId, assignments)).toBe(false);
  });

  it("clamps reels per day and updates channel, mandatory and start", () => {
    let state: MatrixState = matrixReducer({}, { type: "hydrate", topicId, assignments });
    state = matrixReducer(state, { type: "set_reels_per_day", topicId, groupId: "g1", value: 99 });
    expect(state[topicId]?.g1?.reels_per_day).toBe(5);
    state = matrixReducer(state, { type: "set_reels_per_day", topicId, groupId: "g1", value: Number.NaN });
    expect(state[topicId]?.g1?.reels_per_day).toBe(1);
    state = matrixReducer(state, { type: "set_channel", topicId, groupId: "g1", value: "whatsapp" });
    state = matrixReducer(state, { type: "set_mandatory", topicId, groupId: "g1", value: true });
    state = matrixReducer(state, { type: "set_starts_on", topicId, groupId: "g1", value: "2026-11-01" });
    expect(toWriteList(state, topicId)).toEqual([{ group_id: "g1", reels_per_day: 1, channel: "whatsapp", is_mandatory: true, starts_on: "2026-11-01", ends_on: null }]);
  });

  it("ignores updates to unassigned cells and resets to the original", () => {
    let state: MatrixState = matrixReducer({}, { type: "hydrate", topicId, assignments });
    const untouched = matrixReducer(state, { type: "set_channel", topicId, groupId: "nope", value: "teams" });
    expect(untouched).toBe(state);
    state = matrixReducer(state, { type: "set_channel", topicId, groupId: "g1", value: "teams" });
    expect(isDirty(state, topicId, assignments)).toBe(true);
    state = matrixReducer(state, { type: "reset", topicId, assignments });
    expect(isDirty(state, topicId, assignments)).toBe(false);
  });
});
