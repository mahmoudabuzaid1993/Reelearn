import type { Assignment, AssignmentWrite, Channel } from "@/lib/api/types";

export interface Cell {
  groupId: string;
  reels_per_day: number;
  channel: Channel;
  is_mandatory: boolean;
  starts_on: string;
  ends_on: string | null;
}

/** topicId -> groupId -> cell */
export type MatrixState = Record<string, Record<string, Cell>>;

export type MatrixAction =
  | { type: "hydrate"; topicId: string; assignments: Assignment[] }
  | { type: "toggle"; topicId: string; groupId: string; defaults: Omit<Cell, "groupId"> }
  | { type: "set_reels_per_day"; topicId: string; groupId: string; value: number }
  | { type: "set_channel"; topicId: string; groupId: string; value: Channel }
  | { type: "set_mandatory"; topicId: string; groupId: string; value: boolean }
  | { type: "set_starts_on"; topicId: string; groupId: string; value: string }
  | { type: "reset"; topicId: string; assignments: Assignment[] };

export const MIN_REELS_PER_DAY = 1;
export const MAX_REELS_PER_DAY = 5;

export function cellFromAssignment(a: Assignment): Cell {
  return {
    groupId: a.group.id,
    reels_per_day: a.reels_per_day,
    channel: a.channel,
    is_mandatory: a.is_mandatory,
    starts_on: a.starts_on,
    ends_on: a.ends_on ?? null,
  };
}

function rowFrom(assignments: Assignment[]): Record<string, Cell> {
  return Object.fromEntries(assignments.map((a) => [a.group.id, cellFromAssignment(a)]));
}

function clampReels(value: number): number {
  if (Number.isNaN(value)) return MIN_REELS_PER_DAY;
  return Math.min(MAX_REELS_PER_DAY, Math.max(MIN_REELS_PER_DAY, Math.round(value)));
}

function update(state: MatrixState, topicId: string, groupId: string, patch: Partial<Cell>): MatrixState {
  const row = state[topicId] ?? {};
  const cell = row[groupId];
  if (!cell) return state;
  return { ...state, [topicId]: { ...row, [groupId]: { ...cell, ...patch } } };
}

export function matrixReducer(state: MatrixState, action: MatrixAction): MatrixState {
  switch (action.type) {
    case "hydrate":
    case "reset":
      return { ...state, [action.topicId]: rowFrom(action.assignments) };
    case "toggle": {
      const row = state[action.topicId] ?? {};
      if (row[action.groupId]) {
        const next = { ...row };
        delete next[action.groupId];
        return { ...state, [action.topicId]: next };
      }
      return {
        ...state,
        [action.topicId]: { ...row, [action.groupId]: { groupId: action.groupId, ...action.defaults } },
      };
    }
    case "set_reels_per_day":
      return update(state, action.topicId, action.groupId, { reels_per_day: clampReels(action.value) });
    case "set_channel":
      return update(state, action.topicId, action.groupId, { channel: action.value });
    case "set_mandatory":
      return update(state, action.topicId, action.groupId, { is_mandatory: action.value });
    case "set_starts_on":
      return update(state, action.topicId, action.groupId, { starts_on: action.value });
    default:
      return state;
  }
}

export function toWriteList(state: MatrixState, topicId: string): AssignmentWrite[] {
  return Object.values(state[topicId] ?? {}).map((cell) => ({
    group_id: cell.groupId,
    reels_per_day: cell.reels_per_day,
    channel: cell.channel,
    is_mandatory: cell.is_mandatory,
    starts_on: cell.starts_on,
    ends_on: cell.ends_on,
  }));
}

export function isDirty(state: MatrixState, topicId: string, original: Assignment[]): boolean {
  const a = JSON.stringify(toWriteList(state, topicId).sort((x, y) => x.group_id.localeCompare(y.group_id)));
  const b = JSON.stringify(toWriteList({ [topicId]: rowFrom(original) }, topicId).sort((x, y) => x.group_id.localeCompare(y.group_id)));
  return a !== b;
}

export function assignedGroupCount(state: MatrixState, topicId: string): number {
  return Object.keys(state[topicId] ?? {}).length;
}
