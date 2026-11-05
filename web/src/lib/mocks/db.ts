import type { Assignment, AuditEntry, Export, GenerationJob, Group, Integration, Moment, Reel, Source, Tenant, Topic, User } from "@/lib/api/types";
import { assignments, jobs, moments, reels, sources, topics } from "./fixtures/content";
import { audit, exportsList, groups, integrations, tenant, users } from "./fixtures/tenant";

export interface JobRun {
  startedAt: number;
  stepMs: number;
  /** When a retry happened, steps before this index are already done. */
  resumeFrom: number;
}

export interface MockDb {
  tenant: Tenant;
  users: User[];
  groups: Group[];
  topics: Topic[];
  assignments: Assignment[];
  sources: Source[];
  moments: Moment[];
  jobs: GenerationJob[];
  jobRuns: Record<string, JobRun>;
  reels: Reel[];
  exports: Export[];
  integrations: Integration[];
  audit: AuditEntry[];
  challenges: Record<string, string>;
  onboarding: { topicsDone: boolean; deliveryDone: boolean };
}

const STORAGE_KEY = "reelearn.mockdb.v1";

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function freshDb(): MockDb {
  return clone({
    tenant,
    users,
    groups,
    topics,
    assignments,
    sources,
    moments,
    jobs,
    jobRuns: {},
    reels,
    exports: exportsList,
    integrations,
    audit,
    challenges: {},
    onboarding: { topicsDone: true, deliveryDone: true },
  });
}

function load(): MockDb {
  if (typeof window === "undefined") return freshDb();
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw) as MockDb;
  } catch {
    // fall through to a fresh copy
  }
  return freshDb();
}

export let db: MockDb = load();

export function persist(): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(db));
  } catch {
    // storage may be unavailable (private mode); the in-memory copy still works
  }
}

export function resetDb(): MockDb {
  db = freshDb();
  persist();
  return db;
}

let counter = 0;
export function newId(): string {
  counter += 1;
  const rand = Math.floor(Math.random() * 0xffffffff).toString(16).padStart(8, "0");
  return `00000000-0000-4000-9000-${String(counter).padStart(4, "0")}${rand}`;
}

export function nowIso(): string {
  return new Date().toISOString();
}

export function record(entry: Omit<AuditEntry, "id" | "at">): void {
  db.audit.unshift({ id: newId(), at: nowIso(), ...entry });
}
