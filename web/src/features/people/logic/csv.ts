import type { Channel, Role } from "@/lib/api/types";

export const CSV_COLUMNS = ["full_name", "role", "phone", "email", "employee_no", "preferred_language", "location", "job_title", "delivery_channel", "groups"] as const;
export type CsvColumn = (typeof CSV_COLUMNS)[number];

const HEADER_ALIASES: Record<string, CsvColumn> = {
  name: "full_name",
  full_name: "full_name",
  fullname: "full_name",
  "full name": "full_name",
  role: "role",
  phone: "phone",
  mobile: "phone",
  whatsapp: "phone",
  email: "email",
  "e-mail": "email",
  employee_no: "employee_no",
  "employee no": "employee_no",
  employee_id: "employee_no",
  id: "employee_no",
  language: "preferred_language",
  preferred_language: "preferred_language",
  lang: "preferred_language",
  location: "location",
  store: "location",
  job_title: "job_title",
  title: "job_title",
  channel: "delivery_channel",
  delivery_channel: "delivery_channel",
  groups: "groups",
  group: "groups",
};

const ROLES: Role[] = ["admin", "manager", "sme", "learner"];
const CHANNELS: Channel[] = ["app", "whatsapp", "both", "teams"];
const LANGUAGES = new Set(["ar", "en", "ur", "bn", "hi", "tl"]);

/** Minimal RFC 4180 parser: quoted fields, escaped quotes, CRLF. */
export function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let inQuotes = false;
  const src = text.replace(/^﻿/, "");
  for (let i = 0; i < src.length; i += 1) {
    const ch = src[i];
    if (inQuotes) {
      if (ch === '"') {
        if (src[i + 1] === '"') {
          field += '"';
          i += 1;
        } else {
          inQuotes = false;
        }
      } else {
        field += ch;
      }
      continue;
    }
    if (ch === '"') inQuotes = true;
    else if (ch === ",") {
      row.push(field);
      field = "";
    } else if (ch === "\n" || ch === "\r") {
      if (ch === "\r" && src[i + 1] === "\n") i += 1;
      row.push(field);
      rows.push(row);
      row = [];
      field = "";
    } else field += ch;
  }
  if (field.length > 0 || row.length > 0) {
    row.push(field);
    rows.push(row);
  }
  return rows.filter((r) => r.some((c) => c.trim() !== ""));
}

export function mapHeader(raw: string): CsvColumn | null {
  return HEADER_ALIASES[raw.trim().toLowerCase()] ?? null;
}

export interface ImportRowPreview {
  row: number;
  values: Partial<Record<CsvColumn, string>>;
  errors: string[];
  action: "create" | "update" | "skip";
}

export interface ImportPreview {
  columns: (CsvColumn | null)[];
  unmappedHeaders: string[];
  rows: ImportRowPreview[];
  totals: { total: number; create: number; update: number; skip: number; errors: number };
}

export interface ExistingPerson {
  email?: string | null;
  phone?: string | null;
  employee_no?: string | null;
}

export function normalisePhone(raw: string): string | null {
  const digits = raw.replace(/[\s()-]/g, "");
  if (/^\+9665\d{8}$/.test(digits)) return digits;
  if (/^05\d{8}$/.test(digits)) return `+966${digits.slice(1)}`;
  if (/^9665\d{8}$/.test(digits)) return `+${digits}`;
  if (/^5\d{8}$/.test(digits)) return `+966${digits}`;
  if (/^\+\d{8,15}$/.test(digits)) return digits;
  return null;
}

/** Client-side dry run so the admin sees mapping and per-row errors before anything is sent. */
export function previewImport(text: string, existing: ExistingPerson[] = []): ImportPreview {
  const parsed = parseCsv(text);
  const [header = [], ...body] = parsed;
  const columns = header.map(mapHeader);
  const unmappedHeaders = header.filter((_, i) => columns[i] === null);
  const byEmail = new Set(existing.map((p) => p.email?.toLowerCase()).filter(Boolean));
  const byPhone = new Set(existing.map((p) => p.phone).filter(Boolean));
  const byEmployee = new Set(existing.map((p) => p.employee_no).filter(Boolean));
  const seen = new Set<string>();

  const rows: ImportRowPreview[] = body.map((cells, index) => {
    const values: Partial<Record<CsvColumn, string>> = {};
    columns.forEach((col, i) => {
      const cell = cells[i]?.trim();
      if (col && cell) values[col] = cell;
    });
    const errors: string[] = [];
    if (!values.full_name) errors.push("missing_name");
    if (values.role && !ROLES.includes(values.role.toLowerCase() as Role)) errors.push("bad_role");
    if (!values.phone && !values.email) errors.push("missing_contact");
    if (values.phone) {
      const phone = normalisePhone(values.phone);
      if (!phone) errors.push("bad_phone");
      else values.phone = phone;
    }
    if (values.email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(values.email)) errors.push("bad_email");
    if (values.preferred_language && !LANGUAGES.has(values.preferred_language.toLowerCase())) errors.push("bad_language");
    if (values.delivery_channel && !CHANNELS.includes(values.delivery_channel.toLowerCase() as Channel)) errors.push("bad_channel");

    const identity = (values.email?.toLowerCase() ?? values.phone ?? values.employee_no ?? "").trim();
    if (identity && seen.has(identity)) errors.push("duplicate_in_file");
    if (identity) seen.add(identity);

    const exists =
      (values.email && byEmail.has(values.email.toLowerCase())) ||
      (values.phone && byPhone.has(values.phone)) ||
      (values.employee_no && byEmployee.has(values.employee_no));
    const action: ImportRowPreview["action"] = errors.length > 0 ? "skip" : exists ? "update" : "create";
    return { row: index + 2, values, errors, action };
  });

  const totals = {
    total: rows.length,
    create: rows.filter((r) => r.action === "create").length,
    update: rows.filter((r) => r.action === "update").length,
    skip: rows.filter((r) => r.action === "skip").length,
    errors: rows.filter((r) => r.errors.length > 0).length,
  };
  return { columns, unmappedHeaders, rows, totals };
}
