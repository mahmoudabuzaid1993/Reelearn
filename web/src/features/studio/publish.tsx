"use client";

import Link from "next/link";
import { useLocale, useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { useAuth } from "@/lib/auth/provider";
import { useGroups } from "@/lib/api/hooks/people";
import { usePublishReel, useRecordApproval, useReel, useSubmitReview } from "@/lib/api/hooks/reels";
import type { Reel } from "@/lib/api/types";
import { formatDateTime, todayIso } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Checkbox, Input, Textarea } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { useToast } from "@/components/ui/toast";
import { STATUS_TONE } from "@/features/library/library-view";
import { APPROVAL_ROLES, approvalRoleFor, approvalState, publishBlockers, type ApprovalRole } from "./logic/approvals";
import { PhonePreview } from "./phone-preview";
import { publishSchema } from "./studio.schema";

export function Publish({ reelId }: { reelId: string }) {
  const reel = useReel(reelId);
  if (reel.isPending) return <SkeletonRows rows={8} />;
  if (reel.isError) return <ErrorNotice error={reel.error} onRetry={() => void reel.refetch()} />;
  return <PublishForm reel={reel.data} />;
}

function PublishForm({ reel }: { reel: Reel }) {
  const t = useTranslations("studio.publish");
  const tl = useTranslations("library");
  const tv = useTranslations("validation");
  const locale = useLocale();
  const router = useRouter();
  const { toast } = useToast();
  const { user } = useAuth();
  const groups = useGroups();
  const approve = useRecordApproval();
  const publish = usePublishReel();
  const submitReview = useSubmitReview();
  const [groupIds, setGroupIds] = useState<string[]>(reel.audience?.map((g) => g.id) ?? []);
  const [startsOn, setStartsOn] = useState(todayIso());
  const [complianceTag, setComplianceTag] = useState(reel.compliance_tag ?? "");
  const [comment, setComment] = useState("");
  const [errors, setErrors] = useState<Record<string, string>>({});

  const myRole = approvalRoleFor(user?.role ?? "");
  const blockers = publishBlockers(reel, groupIds.length);
  const canPublishNow = blockers.length === 0;
  const alreadyPublished = reel.status === "published";

  const onPublish = () => {
    const parsed = publishSchema.safeParse({ group_ids: groupIds, starts_on: startsOn, compliance_tag: complianceTag });
    if (!parsed.success) {
      setErrors(Object.fromEntries(parsed.error.issues.map((i) => [String(i.path[0]), i.message])));
      return;
    }
    publish.mutate(
      { id: reel.id, ...parsed.data, compliance_tag: parsed.data.compliance_tag || undefined },
      {
        onSuccess: () => {
          toast({ title: t("published"), tone: "success" });
          router.push("/library?status=published");
        },
      },
    );
  };

  return (
    <>
      <PageHeader
        eyebrow={t("step")}
        title={reel.title}
        description={
          <span className="flex items-center gap-2">
            <Badge tone={STATUS_TONE[reel.status]}>{tl(`statuses.${reel.status}`)}</Badge>
            <span>v{reel.version}</span>
            <Link href={`/studio/reels/${reel.id}`} className="underline underline-offset-2">
              {t("backToEditor")}
            </Link>
          </span>
        }
      />
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[1fr_16rem]">
        <div className="flex flex-col gap-4">
          <Card>
            <CardHeader title={t("approvals")} description={t("approvalsHint")} />
            <CardBody className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              {APPROVAL_ROLES.map((role) => (
                <ApprovalCard
                  key={role}
                  role={role}
                  reel={reel}
                  mine={myRole === role}
                  comment={comment}
                  onComment={setComment}
                  pending={approve.isPending}
                  onDecide={(decision) =>
                    approve.mutate(
                      { id: reel.id, decision, comment: comment || undefined },
                      { onSuccess: () => toast({ title: t(`decided.${decision}`), tone: decision === "approved" ? "success" : "neutral" }) },
                    )
                  }
                />
              ))}
              {reel.status === "draft" ? (
                <div className="sm:col-span-2">
                  <Button size="sm" loading={submitReview.isPending} onClick={() => submitReview.mutate({ id: reel.id }, { onSuccess: () => toast({ title: t("submitted"), tone: "success" }) })}>
                    {t("submitForReview")}
                  </Button>
                </div>
              ) : null}
              {approve.error ? (
                <div className="sm:col-span-2">
                  <ErrorNotice error={approve.error} />
                </div>
              ) : null}
            </CardBody>
          </Card>

          <Card>
            <CardHeader title={t("audience")} description={t("audienceHint")} />
            <CardBody className="flex flex-col gap-4">
              <fieldset>
                <legend className="mb-1.5 text-[13px] font-medium text-ink">{t("groups")}</legend>
                <div className="flex flex-wrap gap-3">
                  {(groups.data ?? []).map((g) => (
                    <Checkbox
                      key={g.id}
                      label={`${g.name} (${g.member_count})`}
                      checked={groupIds.includes(g.id)}
                      onChange={(e) => setGroupIds((p) => (e.target.checked ? [...p, g.id] : p.filter((x) => x !== g.id)))}
                    />
                  ))}
                </div>
                {errors.group_ids ? <p role="alert" className="mt-1 text-xs font-medium text-ink">{tv("audience")}</p> : null}
              </fieldset>
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
                <Field label={t("startsOn")} hint={t("startsOnHint")} error={errors.starts_on && tv("date")}>
                  {({ id, describedBy, invalid }) => <Input id={id} type="date" dir="ltr" aria-describedby={describedBy} aria-invalid={invalid} value={startsOn} onChange={(e) => setStartsOn(e.target.value)} />}
                </Field>
                <Field label={t("complianceTag")} hint={t("complianceHint")}>
                  {({ id, describedBy }) => <Input id={id} dir="ltr" aria-describedby={describedBy} value={complianceTag} onChange={(e) => setComplianceTag(e.target.value)} placeholder="HRDF-SAF" />}
                </Field>
              </div>
            </CardBody>
          </Card>

          <Card>
            <CardBody className="flex flex-wrap items-center justify-between gap-3">
              <div className="text-sm">
                {alreadyPublished ? (
                  <p className="text-slate">{t("alreadyPublished", { at: formatDateTime(reel.published_at, locale) })}</p>
                ) : canPublishNow ? (
                  <p className="font-medium text-ink">{t("ready")}</p>
                ) : (
                  <ul className="flex flex-col gap-0.5 text-slate" aria-label={t("blockedBy")}>
                    {blockers.map((b) => (
                      <li key={b}>· {t(`blockers.${b}`)}</li>
                    ))}
                  </ul>
                )}
              </div>
              <Button variant="primary" disabled={!canPublishNow || alreadyPublished} loading={publish.isPending} onClick={onPublish} data-testid="publish-button">
                {t("publish")}
              </Button>
              {publish.error ? (
                <div className="basis-full">
                  <ErrorNotice error={publish.error} />
                </div>
              ) : null}
            </CardBody>
          </Card>
        </div>
        <aside className="lg:sticky lg:top-6 lg:self-start">
          <PhonePreview reel={reel} />
        </aside>
      </div>
    </>
  );
}

function ApprovalCard({ role, reel, mine, comment, onComment, pending, onDecide }: { role: ApprovalRole; reel: Reel; mine: boolean; comment: string; onComment: (v: string) => void; pending: boolean; onDecide: (d: "approved" | "changes_requested" | "rejected") => void }) {
  const t = useTranslations("studio.publish");
  const locale = useLocale();
  const { toast } = useToast();
  const state = approvalState(reel, role);
  const tone = state.decision === "approved" ? "mint" : state.decision === "pending" ? "neutral" : "ink";
  return (
    <div className="flex flex-col gap-2 rounded-[10px] border border-line p-3" data-testid={`approval-${role}`}>
      <div className="flex items-center justify-between">
        <span className="font-medium text-ink">{t(`roles.${role}`)}</span>
        <Badge tone={tone}>{t(`decisions.${state.decision}`)}</Badge>
      </div>
      {state.reviewerName ? (
        <p className="text-xs text-slate">
          {state.reviewerName} · {formatDateTime(state.decidedAt, locale)}
          {state.comment ? ` · "${state.comment}"` : ""}
        </p>
      ) : null}
      {mine && reel.status !== "published" && reel.status !== "archived" ? (
        <>
          <Textarea aria-label={t("comment")} rows={2} value={comment} onChange={(e) => onComment(e.target.value)} placeholder={t("commentPlaceholder")} />
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="secondary" loading={pending} onClick={() => onDecide("approved")} disabled={state.decision === "approved"}>
              {t("approve")}
            </Button>
            <Button size="sm" loading={pending} onClick={() => onDecide("changes_requested")}>
              {t("requestChanges")}
            </Button>
          </div>
        </>
      ) : state.decision === "pending" ? (
        <div className="flex items-center justify-between gap-2 text-xs text-slate">
          <span>{t("awaitingRole", { role: t(`roles.${role}`) })}</span>
          <Button size="sm" variant="quiet" onClick={() => toast({ title: t("nudged", { role: t(`roles.${role}`) }) })}>
            {t("nudge")}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
