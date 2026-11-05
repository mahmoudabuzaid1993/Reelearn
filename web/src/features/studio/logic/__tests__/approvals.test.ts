import { describe, expect, it } from "vitest";
import { approvalRoleFor, approvalState, awaitingLabel, canPublish, isApprovedBy, missingApprovals, publishBlockers } from "../approvals";

const base = { version: 2, status: "in_review" as const, kind: "ai_avatar" as const, consent_confirmed: true };
const ld = { role: "ld", decision: "approved", reviewer_name: "Noura", decided_at: "2026-10-20T10:00:00Z", reel_version: 2 };
const sme = { role: "sme", decision: "approved", reviewer_name: "Hana", decided_at: "2026-10-20T11:00:00Z", reel_version: 2 };

describe("approval gating", () => {
  it("blocks publish until both L&D and SME approve", () => {
    expect(publishBlockers({ ...base, approvals: [] }, 1)).toEqual(["awaiting_ld", "awaiting_sme"]);
    expect(publishBlockers({ ...base, approvals: [ld] }, 1)).toEqual(["awaiting_sme"]);
    expect(publishBlockers({ ...base, approvals: [ld, sme] }, 1)).toEqual([]);
    expect(canPublish({ ...base, approvals: [ld, sme] }, 1)).toBe(true);
  });

  it("ignores approvals from an older version", () => {
    const stale = { ...sme, reel_version: 1 };
    expect(isApprovedBy({ version: 2, approvals: [ld, stale] }, "sme")).toBe(false);
    expect(missingApprovals({ version: 2, approvals: [ld, stale] })).toEqual(["sme"]);
  });

  it("uses the latest decision per role", () => {
    const changes = { ...sme, decision: "changes_requested", decided_at: "2026-10-21T09:00:00Z" };
    expect(approvalState({ version: 2, approvals: [sme, changes] }, "sme").decision).toBe("changes_requested");
    expect(approvalState({ version: 2, approvals: [changes, sme] }, "sme").decision).toBe("changes_requested");
    const reApproved = { ...sme, decided_at: "2026-10-22T09:00:00Z" };
    expect(approvalState({ version: 2, approvals: [changes, reApproved] }, "sme").decision).toBe("approved");
  });

  it("requires an audience and consent for recorded kinds", () => {
    expect(publishBlockers({ ...base, approvals: [ld, sme] }, 0)).toEqual(["no_audience"]);
    expect(publishBlockers({ ...base, kind: "podcast_cut", consent_confirmed: false, approvals: [ld, sme] }, 1)).toEqual(["consent_required"]);
    expect(publishBlockers({ ...base, status: "archived", approvals: [ld, sme] }, 1)).toContain("archived");
  });

  it("maps console roles to approval roles", () => {
    expect(approvalRoleFor("admin")).toBe("ld");
    expect(approvalRoleFor("manager")).toBe("ld");
    expect(approvalRoleFor("sme")).toBe("sme");
    expect(approvalRoleFor("learner")).toBeNull();
  });

  it("derives the library 'awaiting' label", () => {
    expect(awaitingLabel({ ...base, approvals: [] })).toBe("both");
    expect(awaitingLabel({ ...base, approvals: [ld] })).toBe("sme");
    expect(awaitingLabel({ ...base, approvals: [ld, sme] })).toBeNull();
    expect(awaitingLabel({ ...base, status: "published", approvals: [] })).toBeNull();
  });
});
