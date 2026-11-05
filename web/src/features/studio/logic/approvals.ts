import type { Reel, ReelApproval } from "@/lib/api/types";

export const APPROVAL_ROLES = ["ld", "sme"] as const;
export type ApprovalRole = (typeof APPROVAL_ROLES)[number];

export interface ApprovalState {
  role: ApprovalRole;
  decision: "approved" | "changes_requested" | "rejected" | "pending";
  reviewerName: string | null;
  comment: string | null;
  decidedAt: string | null;
}

/** Only approvals recorded against the reel's current version count. */
export function approvalsForVersion(reel: Pick<Reel, "version" | "approvals">): ReelApproval[] {
  return (reel.approvals ?? []).filter((a) => (a.reel_version ?? reel.version) === reel.version);
}

export function approvalState(reel: Pick<Reel, "version" | "approvals">, role: ApprovalRole): ApprovalState {
  const latest = approvalsForVersion(reel)
    .filter((a) => a.role === role)
    .sort((a, b) => (b.decided_at ?? "").localeCompare(a.decided_at ?? ""))[0];
  if (!latest) return { role, decision: "pending", reviewerName: null, comment: null, decidedAt: null };
  const decision = latest.decision;
  return {
    role,
    decision:
      decision === "approved" || decision === "changes_requested" || decision === "rejected" ? decision : "pending",
    reviewerName: latest.reviewer_name ?? null,
    comment: latest.comment ?? null,
    decidedAt: latest.decided_at ?? null,
  };
}

export function isApprovedBy(reel: Pick<Reel, "version" | "approvals">, role: ApprovalRole): boolean {
  return approvalState(reel, role).decision === "approved";
}

export function missingApprovals(reel: Pick<Reel, "version" | "approvals">): ApprovalRole[] {
  return APPROVAL_ROLES.filter((role) => !isApprovedBy(reel, role));
}

export type PublishBlocker = "awaiting_ld" | "awaiting_sme" | "consent_required" | "archived" | "no_audience";

/** Why the publish button is disabled; empty means publish is allowed. */
export function publishBlockers(
  reel: Pick<Reel, "version" | "approvals" | "status" | "kind" | "consent_confirmed">,
  audienceCount: number,
): PublishBlocker[] {
  const blockers: PublishBlocker[] = [];
  if (reel.status === "archived") blockers.push("archived");
  if (!isApprovedBy(reel, "ld")) blockers.push("awaiting_ld");
  if (!isApprovedBy(reel, "sme")) blockers.push("awaiting_sme");
  if ((reel.kind === "podcast_cut" || reel.kind === "sme_recorded") && !reel.consent_confirmed) {
    blockers.push("consent_required");
  }
  if (audienceCount < 1) blockers.push("no_audience");
  return blockers;
}

export function canPublish(
  reel: Pick<Reel, "version" | "approvals" | "status" | "kind" | "consent_confirmed">,
  audienceCount: number,
): boolean {
  return publishBlockers(reel, audienceCount).length === 0;
}

/** Which role the signed-in user approves as; admins act as L&D. */
export function approvalRoleFor(role: string): ApprovalRole | null {
  if (role === "admin" || role === "manager") return "ld";
  if (role === "sme") return "sme";
  return null;
}

/** The "awaiting" label used by the library list. */
export function awaitingLabel(reel: Pick<Reel, "version" | "approvals" | "status">): "ld" | "sme" | "both" | null {
  if (reel.status !== "in_review") return null;
  const missing = missingApprovals(reel);
  if (missing.length === 0) return null;
  if (missing.length === 2) return "both";
  return missing[0] ?? null;
}
