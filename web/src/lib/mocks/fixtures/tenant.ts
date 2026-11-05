import type { Tenant, Me, User, Group, Integration, Dashboard, ReportOverview, ReportRow, Export, AuditEntry } from "@/lib/api/types";
import { IDS } from "./ids";

export const CREDITS = 1200;

export const tenant: Tenant = {
  id: IDS.tenant,
  slug: "alwadi",
  name: "Al Wadi Retail",
  logo_url: null,
  languages: ["ar", "en", "ur", "bn"],
  region: "ksa",
  default_language: "ar",
  timezone: "Asia/Riyadh",
  credits: CREDITS,
  settings: {
    reels_per_day: 2,
    max_reviews_per_day: 1,
    delivery_windows: [
      { days: ["sun", "mon", "tue", "wed", "thu"], start: "08:30", end: "11:30" },
      { days: ["sun", "mon", "tue", "wed", "thu"], start: "16:00", end: "19:00" },
    ],
    prayer_aware: true,
    ramadan_aware: true,
    weekend_days: ["fri", "sat"],
    retention_months: 36,
    streak_freezes_per_week: 1,
  },
};

export const groups: Group[] = [
  { id: IDS.groups.store12, name: "Store 12 · Jeddah", kind: "location", member_count: 2 },
  { id: IDS.groups.warehouse, name: "Jeddah warehouse", kind: "location", member_count: 2 },
  { id: IDS.groups.allStaff, name: "All staff", kind: "custom", member_count: 7 },
];

const g = (id: string) => ({ id, name: groups.find((x) => x.id === id)?.name ?? "" });

export const users: User[] = [
  {
    id: IDS.users.noura,
    role: "admin",
    status: "active",
    full_name: "Noura Al-Harbi",
    email: "admin@alwadi.example",
    phone_e164: "+966500000001",
    employee_no: "AW-0001",
    preferred_language: "ar",
    manager_id: null,
    location: "Head office · Jeddah",
    job_title: "L&D Manager",
    delivery_channel: "app",
    delivery_slot: "shift_start",
    groups: [g(IDS.groups.allStaff)],
    last_seen_at: "2026-10-29T06:12:00Z",
    created_at: "2026-09-01T08:00:00Z",
  },
  {
    id: IDS.users.khalid,
    role: "manager",
    status: "active",
    full_name: "Khalid Mansour",
    email: "khalid@alwadi.example",
    phone_e164: "+966500000002",
    employee_no: "AW-0002",
    preferred_language: "ar",
    manager_id: null,
    location: "Store 12 · Jeddah",
    job_title: "Store manager",
    delivery_channel: "both",
    delivery_slot: "shift_start",
    groups: [g(IDS.groups.store12), g(IDS.groups.allStaff)],
    last_seen_at: "2026-10-29T05:40:00Z",
    created_at: "2026-09-01T08:00:00Z",
  },
  {
    id: IDS.users.hana,
    role: "sme",
    status: "active",
    full_name: "Hana Yusuf",
    email: "hana@alwadi.example",
    phone_e164: "+966500000003",
    employee_no: "AW-0003",
    preferred_language: "en",
    manager_id: null,
    location: "Head office · Jeddah",
    job_title: "Compliance lead",
    delivery_channel: "app",
    delivery_slot: "break",
    groups: [g(IDS.groups.allStaff)],
    last_seen_at: "2026-10-28T13:05:00Z",
    created_at: "2026-09-02T08:00:00Z",
  },
  {
    id: IDS.users.sara,
    role: "learner",
    status: "active",
    full_name: "Sara Al-Qahtani",
    email: null,
    phone_e164: "+966500000004",
    employee_no: "AW-0104",
    preferred_language: "ar",
    manager_id: IDS.users.khalid,
    location: "Store 12 · Jeddah",
    job_title: "Cashier",
    delivery_channel: "app",
    delivery_slot: "shift_start",
    groups: [g(IDS.groups.store12), g(IDS.groups.allStaff)],
    last_seen_at: "2026-10-29T05:55:00Z",
    created_at: "2026-09-03T08:00:00Z",
  },
  {
    id: IDS.users.rizwan,
    role: "learner",
    status: "active",
    full_name: "Rizwan Ahmed",
    email: null,
    phone_e164: "+966500000005",
    employee_no: "AW-0105",
    preferred_language: "ur",
    manager_id: IDS.users.khalid,
    location: "Jeddah warehouse",
    job_title: "Picker",
    delivery_channel: "whatsapp",
    delivery_slot: "shift_start",
    groups: [g(IDS.groups.warehouse), g(IDS.groups.allStaff)],
    last_seen_at: "2026-10-29T04:20:00Z",
    created_at: "2026-09-03T08:00:00Z",
  },
  {
    id: IDS.users.mohammad,
    role: "learner",
    status: "active",
    full_name: "Mohammad A.",
    email: null,
    phone_e164: "+966500000006",
    employee_no: "AW-0106",
    preferred_language: "bn",
    manager_id: IDS.users.khalid,
    location: "Jeddah warehouse",
    job_title: "Forklift operator",
    delivery_channel: "whatsapp",
    delivery_slot: "break",
    groups: [g(IDS.groups.warehouse), g(IDS.groups.allStaff)],
    last_seen_at: "2026-10-27T10:10:00Z",
    created_at: "2026-09-03T08:00:00Z",
  },
  {
    id: IDS.users.fatimah,
    role: "learner",
    status: "invited",
    full_name: "Fatimah Noor",
    email: "fatimah@alwadi.example",
    phone_e164: "+966500000007",
    employee_no: "AW-0107",
    preferred_language: "en",
    manager_id: IDS.users.khalid,
    location: "Store 12 · Jeddah",
    job_title: "Sales associate",
    delivery_channel: "app",
    delivery_slot: "shift_end",
    groups: [g(IDS.groups.store12), g(IDS.groups.allStaff)],
    last_seen_at: null,
    created_at: "2026-10-20T08:00:00Z",
  },
];

export function meFor(user: User): Me {
  return {
    id: user.id,
    tenant: { id: tenant.id, slug: tenant.slug, name: tenant.name, logo_url: null, languages: tenant.languages, region: tenant.region },
    role: user.role,
    full_name: user.full_name,
    first_name: user.full_name.split(" ")[0],
    phone_e164: user.phone_e164 ?? null,
    email: user.email ?? null,
    preferred_language: user.preferred_language,
    ui_language: user.preferred_language === "ar" ? "ar" : "en",
    delivery_channel: user.delivery_channel,
    delivery_slot: user.delivery_slot ?? "shift_start",
    captions_on: true,
    reminders_on: true,
    extra_reels_on: false,
    paused_until: null,
    is_first_open: false,
  };
}

export const integrations: Integration[] = [
  { kind: "hris_jisr", status: "connected", last_sync_at: "2026-10-29T03:00:00Z", last_error: null, public_config: { company_id: "alwadi-retail", sync_hour: 6 } },
  { kind: "hris_bayzat", status: "disconnected", last_sync_at: null, last_error: null, public_config: {} },
  { kind: "sso_entra", status: "connected", last_sync_at: null, last_error: null, public_config: { tenant_id: "7a1f…c3", domain: "alwadi.example" } },
  { kind: "whatsapp", status: "connected", last_sync_at: "2026-10-29T06:00:00Z", last_error: null, public_config: { phone_number: "+966 55 000 1200", quality: "green" } },
  { kind: "teams", status: "disconnected", last_sync_at: null, last_error: null, public_config: {} },
  { kind: "ai_muse", status: "connected", last_sync_at: null, last_error: null, public_config: { model: "muse-spark-1.3" } },
  { kind: "ai_claude", status: "disconnected", last_sync_at: null, last_error: null, public_config: { model: "claude-sonnet" } },
  { kind: "ai_higgsfield", status: "connected", last_sync_at: null, last_error: null, public_config: {} },
  { kind: "ai_veo", status: "error", last_sync_at: null, last_error: "Quota exceeded for project (429)", public_config: {} },
  { kind: "voice", status: "connected", last_sync_at: null, last_error: null, public_config: { default_voice: "Layla (ar-SA)" } },
  { kind: "asr", status: "connected", last_sync_at: null, last_error: null, public_config: {} },
  { kind: "storage", status: "connected", last_sync_at: null, last_error: null, public_config: { bucket: "reelearn-ksa-1", region: "ksa" } },
];

export const dashboard: Dashboard = {
  weekly_active_pct: 71,
  recall_pct: 78,
  learners_total: 4,
  learners_active: 3,
  hrdf_hours: 12.5,
  credits: CREDITS,
  attention: [
    { kind: "awaiting_sme", text: "2 reels are waiting for SME sign-off", href: "/library?status=in_review" },
    { kind: "invite_pending", text: "Fatimah Noor has not opened her invite", href: `/people?q=Fatimah` },
    { kind: "provider_error", text: "Veo integration reports an error", href: "/settings" },
  ],
  series: Array.from({ length: 30 }, (_, i) => {
    const day = new Date(Date.UTC(2026, 9, 1 + i));
    const weekend = day.getUTCDay() === 5 || day.getUTCDay() === 6;
    return {
      day: day.toISOString().slice(0, 10),
      active: weekend ? 0 : 2 + (i % 3),
      recall_pct: 62 + Math.min(18, Math.round(i * 0.6)),
    };
  }),
};

export const reportOverview: ReportOverview = {
  learners_active: 3,
  reels_completed: 84,
  answers_total: 96,
  recall_pct: 78,
  minutes_on_mandatory: 750,
};

export const reportsByTopic: ReportRow[] = [
  { id: IDS.topics.customerService, name: "Customer service", learners_active: 3, learners_total: 4, recall_pct: 82, reels_completed: 36, minutes_total: 27, ready_pct: 75 },
  { id: IDS.topics.storeSafety, name: "Store safety", learners_active: 3, learners_total: 4, recall_pct: 79, reels_completed: 30, minutes_total: 22.5, ready_pct: 75 },
  { id: IDS.topics.aml, name: "AML", learners_active: 2, learners_total: 4, recall_pct: 71, reels_completed: 18, minutes_total: 13.5, ready_pct: 50 },
];

export const reportsByGroup: ReportRow[] = [
  { id: IDS.groups.store12, name: "Store 12 · Jeddah", learners_active: 1, learners_total: 2, recall_pct: 84, reels_completed: 40, minutes_total: 30, ready_pct: 50 },
  { id: IDS.groups.warehouse, name: "Jeddah warehouse", learners_active: 2, learners_total: 2, recall_pct: 73, reels_completed: 44, minutes_total: 33, ready_pct: 100 },
  { id: IDS.groups.allStaff, name: "All staff", learners_active: 3, learners_total: 4, recall_pct: 78, reels_completed: 84, minutes_total: 63, ready_pct: 75 },
];

export const exportsList: Export[] = [
  { id: IDS.exports.q3, kind: "hrdf_evidence", status: "ready", period_start: "2026-07-01", period_end: "2026-09-30", created_at: "2026-10-02T07:00:00Z", finished_at: "2026-10-02T07:03:00Z" },
];

export const audit: AuditEntry[] = [
  { id: "00000000-0000-4000-8000-001100000001", actor: { id: IDS.users.noura, full_name: "Noura Al-Harbi" }, action: "reel.published", entity: "reel", entity_id: IDS.reels.customerName, diff: { status: ["approved", "published"], audience: [[], ["Store 12 · Jeddah"]] } as never, at: "2026-10-12T09:15:00Z" },
  { id: "00000000-0000-4000-8000-001100000002", actor: { id: IDS.users.hana, full_name: "Hana Yusuf" }, action: "reel.approved", entity: "reel", entity_id: IDS.reels.cashReports, diff: { approvals: ["sme: approved"] } as never, at: "2026-10-20T11:02:00Z" },
  { id: "00000000-0000-4000-8000-001100000003", actor: { id: IDS.users.noura, full_name: "Noura Al-Harbi" }, action: "topic.assignment.updated", entity: "topic", entity_id: IDS.topics.storeSafety, diff: { reels_per_day: [1, 2] } as never, at: "2026-10-22T08:30:00Z" },
  { id: "00000000-0000-4000-8000-001100000004", actor: null, action: "hris.sync", entity: "integration", entity_id: null, diff: { created: 0, updated: 2, skipped: 5 } as never, at: "2026-10-29T03:00:00Z" },
  { id: "00000000-0000-4000-8000-001100000005", actor: { id: IDS.users.noura, full_name: "Noura Al-Harbi" }, action: "user.invited", entity: "user", entity_id: IDS.users.fatimah, diff: { status: [null, "invited"] } as never, at: "2026-10-20T08:00:00Z" },
  { id: "00000000-0000-4000-8000-001100000006", actor: { id: IDS.users.noura, full_name: "Noura Al-Harbi" }, action: "generation_job.created", entity: "generation_job", entity_id: IDS.jobs.done, diff: { cost_credits: 54 } as never, at: "2026-10-10T10:00:00Z" },
];

export const hrisSyncLog = [
  { at: "2026-10-29T03:00:00Z", provider: "Jisr", created: 0, updated: 2, skipped: 5, status: "ok" },
  { at: "2026-10-28T03:00:00Z", provider: "Jisr", created: 1, updated: 0, skipped: 6, status: "ok" },
  { at: "2026-10-27T03:00:00Z", provider: "Jisr", created: 0, updated: 0, skipped: 7, status: "ok" },
  { at: "2026-10-26T03:00:00Z", provider: "Jisr", created: 0, updated: 0, skipped: 0, status: "error", message: "Jisr token expired (401)" },
] as const;
