"use client";

import { useLocale, useTranslations } from "next-intl";
import { useEffect, useMemo, useReducer, useState } from "react";
import { useGroups } from "@/lib/api/hooks/people";
import { useAllAssignments, useSaveAssignments, useTopics } from "@/lib/api/hooks/topics";
import type { Channel, Topic } from "@/lib/api/types";
import { formatPct, todayIso } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Input, Select } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { useToast } from "@/components/ui/toast";
import { assignedGroupCount, isDirty, matrixReducer, toWriteList, type MatrixState } from "./logic/matrix";
import { TopicDialog } from "./topic-dialog";

const CHANNELS: Channel[] = ["app", "whatsapp", "both", "teams"];

export function TopicsView() {
  const t = useTranslations("topics");
  const tc = useTranslations("common");
  const locale = useLocale();
  const { toast } = useToast();
  const topics = useTopics();
  const groups = useGroups();
  const topicIds = useMemo(() => (topics.data ?? []).map((x) => x.id), [topics.data]);
  const assignmentQueries = useAllAssignments(topicIds);
  const save = useSaveAssignments();
  const [state, dispatch] = useReducer(matrixReducer, {} as MatrixState);
  const [editing, setEditing] = useState<Topic | null | "new">(null);
  const [expanded, setExpanded] = useState<string | null>(null);

  const allLoaded = assignmentQueries.every((q) => q.data);
  useEffect(() => {
    assignmentQueries.forEach((q, i) => {
      const topicId = topicIds[i];
      if (q.data && topicId && !state[topicId]) dispatch({ type: "hydrate", topicId, assignments: q.data });
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [allLoaded, topicIds.join(",")]);

  if (topics.isPending || groups.isPending) return <SkeletonRows rows={8} />;
  if (topics.isError) return <ErrorNotice error={topics.error} onRetry={() => void topics.refetch()} />;
  if (groups.isError) return <ErrorNotice error={groups.error} onRetry={() => void groups.refetch()} />;

  return (
    <>
      <PageHeader title={t("title")} description={t("subtitle")} actions={<Button variant="primary" onClick={() => setEditing("new")}>{t("newTopic")}</Button>} />
      {topics.data.length === 0 ? (
        <EmptyState title={t("empty")} description={t("emptyHint")} action={<Button variant="primary" onClick={() => setEditing("new")}>{t("newTopic")}</Button>} />
      ) : (
        <Card>
          <CardHeader title={t("matrix")} description={t("matrixHint")} />
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-sm">
              <thead>
                <tr>
                  <th scope="col" className="sticky start-0 z-10 border-b border-line bg-surface-muted/60 px-4 py-2.5 text-start text-xs font-semibold uppercase tracking-wide text-slate">
                    {t("topic")}
                  </th>
                  {groups.data.map((g) => (
                    <th key={g.id} scope="col" className="border-b border-line bg-surface-muted/60 px-4 py-2.5 text-start text-xs font-semibold uppercase tracking-wide text-slate">
                      {g.name}
                      <span className="ms-1 font-normal normal-case text-slate/80">({g.member_count})</span>
                    </th>
                  ))}
                  <th scope="col" className="border-b border-line bg-surface-muted/60 px-4 py-2.5 text-end text-xs font-semibold uppercase tracking-wide text-slate">
                    {t("recall")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {topics.data.map((topic, i) => {
                  const original = assignmentQueries[i]?.data ?? [];
                  const dirty = state[topic.id] ? isDirty(state, topic.id, original) : false;
                  const open = expanded === topic.id;
                  return (
                    <tr key={topic.id} className="align-top">
                      <th scope="row" className="sticky start-0 z-10 border-b border-line bg-surface px-4 py-3 text-start font-normal">
                        <div className="flex flex-col gap-1">
                          <button type="button" className="text-start font-medium text-ink underline-offset-2 hover:underline" onClick={() => setEditing(topic)}>
                            {locale === "ar" && topic.name_ar ? topic.name_ar : topic.name}
                          </button>
                          <div className="flex flex-wrap gap-1">
                            {topic.is_mandatory ? <Badge tone="ink">{t("mandatory")}</Badge> : null}
                            {topic.compliance_tag ? <Badge tone="outline">{topic.compliance_tag}</Badge> : null}
                          </div>
                          <p className="text-xs text-slate">
                            {t("reelCounts", { published: topic.reel_counts.published ?? 0, review: topic.reel_counts.in_review ?? 0, draft: topic.reel_counts.draft ?? 0 })}
                          </p>
                          <div className="mt-1 flex items-center gap-2">
                            <button type="button" aria-expanded={open} onClick={() => setExpanded(open ? null : topic.id)} className="text-xs text-slate underline underline-offset-2">
                              {open ? t("hideDetails") : t("showDetails")}
                            </button>
                            {dirty ? (
                              <>
                                <Button
                                  size="sm"
                                  variant="primary"
                                  loading={save.isPending && save.variables?.topicId === topic.id}
                                  onClick={() =>
                                    save.mutate(
                                      { topicId: topic.id, body: toWriteList(state, topic.id) },
                                      {
                                        onSuccess: (data) => {
                                          dispatch({ type: "hydrate", topicId: topic.id, assignments: data });
                                          toast({ title: t("saved"), tone: "success" });
                                        },
                                      },
                                    )
                                  }
                                >
                                  {tc("save")}
                                </Button>
                                <Button size="sm" variant="quiet" onClick={() => dispatch({ type: "reset", topicId: topic.id, assignments: original })}>
                                  {tc("reset")}
                                </Button>
                              </>
                            ) : null}
                          </div>
                        </div>
                      </th>
                      {groups.data.map((g) => {
                        const cell = state[topic.id]?.[g.id];
                        const cellLabel = t("cellLabel", { topic: topic.name, group: g.name });
                        return (
                          <td key={g.id} className="border-b border-line px-4 py-3">
                            <label className="flex cursor-pointer items-center gap-2">
                              <input
                                type="checkbox"
                                className="size-4 accent-ink"
                                aria-label={cellLabel}
                                checked={Boolean(cell)}
                                onChange={() =>
                                  dispatch({
                                    type: "toggle",
                                    topicId: topic.id,
                                    groupId: g.id,
                                    defaults: { reels_per_day: 2, channel: "app", is_mandatory: topic.is_mandatory ?? false, starts_on: todayIso(), ends_on: null },
                                  })
                                }
                              />
                              <span className="text-xs text-slate">{cell ? t("cellSummary", { n: cell.reels_per_day, channel: tc(`channels.${cell.channel}`) }) : t("notAssigned")}</span>
                            </label>
                            {cell && open ? (
                              <div className="mt-2 flex flex-col gap-2">
                                <label className="flex items-center justify-between gap-2 text-xs text-slate">
                                  {t("reelsPerDay")}
                                  <Input type="number" min={1} max={5} className="h-8 w-16" value={cell.reels_per_day} onChange={(e) => dispatch({ type: "set_reels_per_day", topicId: topic.id, groupId: g.id, value: Number(e.target.value) })} />
                                </label>
                                <label className="flex items-center justify-between gap-2 text-xs text-slate">
                                  {t("channel")}
                                  <Select className="h-8 w-28" value={cell.channel} onChange={(e) => dispatch({ type: "set_channel", topicId: topic.id, groupId: g.id, value: e.target.value as Channel })}>
                                    {CHANNELS.map((c) => (
                                      <option key={c} value={c}>
                                        {tc(`channels.${c}`)}
                                      </option>
                                    ))}
                                  </Select>
                                </label>
                                <label className="flex items-center justify-between gap-2 text-xs text-slate">
                                  {t("startsOn")}
                                  <Input type="date" dir="ltr" className="h-8 w-36" value={cell.starts_on} onChange={(e) => dispatch({ type: "set_starts_on", topicId: topic.id, groupId: g.id, value: e.target.value })} />
                                </label>
                                <label className="flex items-center gap-2 text-xs text-slate">
                                  <input type="checkbox" className="size-3.5 accent-ink" checked={cell.is_mandatory} onChange={(e) => dispatch({ type: "set_mandatory", topicId: topic.id, groupId: g.id, value: e.target.checked })} />
                                  {t("mandatory")}
                                </label>
                              </div>
                            ) : null}
                          </td>
                        );
                      })}
                      <td className="border-b border-line px-4 py-3 text-end tabular-nums">
                        {formatPct(topic.recall_pct, locale)}
                        <p className="text-xs text-slate">{t("groupsAssigned", { n: assignedGroupCount(state, topic.id) })}</p>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
          <CardBody className="text-xs text-slate">{t("footnote")}</CardBody>
        </Card>
      )}
      {save.error ? <div className="mt-4"><ErrorNotice error={save.error} /></div> : null}
      <TopicDialog topic={editing === "new" ? null : editing} open={editing !== null} onClose={() => setEditing(null)} />
    </>
  );
}
