"use client";

import Link from "next/link";
import { useLocale, useTranslations } from "next-intl";
import { useRegenerateReel, useSubmitReview } from "@/lib/api/hooks/reels";
import { useJob, useStoryboard } from "@/lib/api/hooks/studio";
import type { Reel } from "@/lib/api/types";
import { formatDuration, formatNumber } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { useToast } from "@/components/ui/toast";
import { STATUS_TONE } from "@/features/library/library-view";
import { PhonePreview } from "./phone-preview";

const REGEN_COST = { visual: 12, voice: 4 } as const;

export function Storyboard({ jobId }: { jobId: string }) {
  const t = useTranslations("studio.storyboard");
  const tl = useTranslations("library");
  const locale = useLocale();
  const { toast } = useToast();
  const job = useJob(jobId);
  const reels = useStoryboard(jobId);
  const regenerate = useRegenerateReel();
  const submit = useSubmitReview();

  if (reels.isPending || job.isPending) return <SkeletonRows rows={8} />;
  if (reels.isError) return <ErrorNotice error={reels.error} onRetry={() => void reels.refetch()} />;
  if (job.isError) return <ErrorNotice error={job.error} onRetry={() => void job.refetch()} />;
  const drafts = reels.data.filter((r) => r.status === "draft");

  return (
    <>
      <PageHeader
        eyebrow={t("step")}
        title={t("title")}
        description={t("subtitle", { n: reels.data.length, credits: formatNumber(job.data.cost_credits, locale) })}
        actions={
          <Button
            variant="primary"
            disabled={drafts.length === 0}
            loading={submit.isPending}
            onClick={async () => {
              for (const r of drafts) await submit.mutateAsync({ id: r.id });
              toast({ title: t("sentToReview", { n: drafts.length }), tone: "success" });
            }}
          >
            {t("sendAll")}
          </Button>
        }
      />
      {reels.data.length === 0 ? (
        <EmptyState title={t("empty")} description={t("emptyHint")} action={<Link href={`/studio/jobs/${jobId}`} className="underline">{t("backToJob")}</Link>} />
      ) : (
        <div className="flex flex-col gap-4">
          {reels.data.map((reel, i) => (
            <StoryboardCard key={reel.id} index={i + 1} reel={reel} onRegenerate={(what) => regenerate.mutate({ id: reel.id, what: [what] }, { onSuccess: () => toast({ title: t("regenQueued"), description: t("regenCost", { n: REGEN_COST[what] }) }) })} regenerating={regenerate.isPending && regenerate.variables?.id === reel.id} />
          ))}
        </div>
      )}
      {regenerate.error ? <div className="mt-4"><ErrorNotice error={regenerate.error} /></div> : null}
      <p className="mt-4 text-xs text-slate">{t("statusLegend", { status: tl("statuses.in_review") })}</p>
    </>
  );
}

function StoryboardCard({ index, reel, onRegenerate, regenerating }: { index: number; reel: Reel; onRegenerate: (what: "visual" | "voice") => void; regenerating: boolean }) {
  const t = useTranslations("studio.storyboard");
  const tl = useTranslations("library");
  return (
    <Card>
      <CardHeader
        title={
          <span className="flex items-center gap-2">
            <span className="text-slate">{index}.</span> {reel.title}
            <Badge tone={STATUS_TONE[reel.status]}>{tl(`statuses.${reel.status}`)}</Badge>
          </span>
        }
        description={`${tl(`kinds.${reel.kind}`)} · ${formatDuration(reel.duration_ms)} · ${reel.language}`}
        actions={
          <>
            <Button size="sm" loading={regenerating} onClick={() => onRegenerate("visual")}>
              {t("regenVisual", { n: REGEN_COST.visual })}
            </Button>
            <Button size="sm" loading={regenerating} onClick={() => onRegenerate("voice")}>
              {t("regenVoice", { n: REGEN_COST.voice })}
            </Button>
            <Link href={`/studio/reels/${reel.id}`} className="inline-flex h-8 items-center rounded-[10px] border border-ink bg-ink px-3 font-display text-[13px] font-medium text-white hover:bg-night">
              {t("edit")}
            </Link>
          </>
        }
      />
      <CardBody className="grid grid-cols-1 gap-4 md:grid-cols-[10rem_1fr_1fr]">
        <PhonePreview reel={reel} compact />
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-wide text-slate">{t("script")}</h3>
          <p className="mt-1 whitespace-pre-wrap text-sm leading-relaxed text-ink">{reel.script}</p>
          {reel.source_citation ? <p className="mt-2 text-xs text-slate">{t("source")}: {reel.source_citation}</p> : null}
        </div>
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-wide text-slate">{t("question")}</h3>
          {reel.question ? (
            <>
              <p className="mt-1 text-sm font-medium text-ink">{reel.question.prompt}</p>
              <ul className="mt-2 flex flex-col gap-1 text-sm">
                {reel.question.options.map((o) => (
                  <li key={o.key} className={`flex items-center gap-2 rounded-md border px-2 py-1 ${o.key === reel.question?.correct_key ? "border-mint bg-mint-soft" : "border-line"}`}>
                    <span className="text-xs font-semibold text-slate">{o.key}</span> {o.text}
                  </li>
                ))}
              </ul>
            </>
          ) : (
            <p className="mt-1 text-sm text-slate">{t("noQuestion")}</p>
          )}
        </div>
      </CardBody>
    </Card>
  );
}
