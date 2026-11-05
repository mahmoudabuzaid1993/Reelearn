import { describe, expect, it } from "vitest";
import { mapHeader, normalisePhone, parseCsv, previewImport } from "../csv";

describe("CSV parsing", () => {
  it("handles quoted fields, escaped quotes, CRLF and a BOM", () => {
    const rows = parseCsv('﻿name,role\r\n"Al-Harbi, Noura",admin\r\n"She said ""hi""",learner\r\n');
    expect(rows).toEqual([
      ["name", "role"],
      ["Al-Harbi, Noura", "admin"],
      ['She said "hi"', "learner"],
    ]);
  });

  it("maps header aliases", () => {
    expect(mapHeader("Full Name")).toBe("full_name");
    expect(mapHeader("mobile")).toBe("phone");
    expect(mapHeader("LANG")).toBe("preferred_language");
    expect(mapHeader("shoe size")).toBeNull();
  });

  it("normalises Saudi numbers to E.164", () => {
    expect(normalisePhone("0551234567")).toBe("+966551234567");
    expect(normalisePhone("+966 55 123 4567")).toBe("+966551234567");
    expect(normalisePhone("551234567")).toBe("+966551234567");
    expect(normalisePhone("12345")).toBeNull();
  });
});

describe("import preview (dry run)", () => {
  const csv = `full_name,role,phone,email,language,shoe size
Layla Hassan,learner,0551234567,,ar,38
Omar Siddiqui,learner,,omar@alwadi.example,ur,
,learner,0559999999,,ar,
Bad Role,ceo,0558888888,,ar,
No Contact,learner,,,ar,
Dup One,learner,0557777777,,ar,
Dup Two,learner,0557777777,,ar,
Existing,learner,,admin@alwadi.example,en,`;

  it("classifies rows into create, update and skip with per-row errors", () => {
    const preview = previewImport(csv, [{ email: "admin@alwadi.example" }]);
    expect(preview.unmappedHeaders).toEqual(["shoe size"]);
    expect(preview.totals).toEqual({ total: 8, create: 3, update: 1, skip: 4, errors: 4 });
    const byRow = Object.fromEntries(preview.rows.map((r) => [r.row, r]));
    expect(byRow[2]?.action).toBe("create");
    expect(byRow[2]?.values.phone).toBe("+966551234567");
    expect(byRow[4]?.errors).toContain("missing_name");
    expect(byRow[5]?.errors).toContain("bad_role");
    expect(byRow[6]?.errors).toContain("missing_contact");
    expect(byRow[8]?.errors).toContain("duplicate_in_file");
    expect(byRow[9]?.action).toBe("update");
  });

  it("returns an empty preview for a header-only file", () => {
    const preview = previewImport("full_name,phone\n");
    expect(preview.rows).toEqual([]);
    expect(preview.totals.total).toBe(0);
  });
});
