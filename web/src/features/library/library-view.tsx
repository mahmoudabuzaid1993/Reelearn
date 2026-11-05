"use client";

import Link from "next/link";
import { useLocale, useTranslations } from "next-intl";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { useReels } from "@/lib/api/hooks/reels";
import { useTopics } from "@/lib/api/hooks/topics";
import type { ReelKind, ReelStatus } from "@/lib/api/types";
import { formatDateTime, formatDuration, formatNumber } from "@/lib/format";
import { Badge, type BadgeTone } from "@/components/ui/badge";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Input, Select } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { Table, Td, Th, Tr } from "@/components/ui/table";
import { RecallBar } from "@/features/reports/reports-view";
import { parseFilters, toSearchParams, type LibraryFilters } from "./logic/filters";

const STATUSES: ReelStatus[] = ["draft", "in_review", "approved", "published", "archived", "rejected"];
const KINDS: ReelKind[] = ["ai_avatar", "motion", "broll", "podcast_cut", "sme_recorded"];
export const STATUS_TONE: Record<ReelStatus, BadgeTone> = { draft: "neutral", in_review: "ink", approved: "mint", published: "mint", archived: "outline", rejected: "outline" };

export function LibraryView() {
  const t = useTranslations("library");
  const tc = useTranslations("common");
  const locale = useLocale();
  const router = useRouter();
  const pathname = usePathname();
  const params = useSearchParams();
  const [filters, setFilters] = useState<LibraryFilters>(() => parseFilters(params));
  const [q, setQ] = useState(filters.q ?? "");
  const topics = useTopics();
  const query = useReels(filters);

  const update = useCallback(
    (patch: Partial<LibraryFilters>) => {
      setFilters((f) => {
        const next = { ...f, ...patch };
        router.replace(`${pathname}?${toSearchParams(next).toString()}`);
        return next;
      });
    },
    [router, pathname],
  );

  useEffect(() => {
    const handle = setTimeout(() => {
      if ((q || undefined) !== filters.q) update({ q: q || undefined });
    }, 250);
    return () => clearTimeout(handle);
  }, [q, filters.q, update]);

  return (
    <>
      <PageHeader
        title={t("title")}
        description={t("subtitle")}
        actions={
          <Link href="/studio/new" className="inline-flex h-10 items-center rounded-[10px] border border-coral bg-coral px-4 font-display text-sm font-medium text-white hover:bg-coral-strong">
            {t("makeReels")}
          </Link>
        }
      />
      <div className="mb-4 grid grid-cols-2 gap-3 md:grid-cols-5">
        <Field label={t("search")} className="col-span-2 md:col-span-1">
          {({ id }) => <Input id={id} type="search" value={q} onChange={(e) => setQ(e.target.value)} placeholder={t("searchPlaceholder")} />}
        </Field>
        <Field label={t("status")}>
          {({ id }) => (
            <Select id={id} value={filters.status ?? ""} onChange={(e) => update({ status: e.target.value || undefined })}>
              <option value="">{tc("all")}</option>
              {STATUSES.map((s) => (
                <option key={s} value={s}>
                  {t(`statuses.${s}`)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t("topic")}>
          {({ id }) => (
            <Select id={id} value={filters.topic_id ?? ""} onChange={(e) => update({ topic_id: e.target.value || undefined })}>
              <option value="">{tc("all")}</option>
              {(topics.data ?? []).map((tp) => (
                <option key={tp.id} value={tp.id}>
                  {tp.name}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t("language")}>
          {({ id }) => (
            <Select id={id} value={filters.language ?? ""} onChange={(e) => update({ language: e.target.value || undefined })}>
              <option value="">{tc("all")}</option>
              {["ar", "en", "ur", "bn", "hi", "tl"].map((l) => (
                <option key={l} value={l}>
                  {tc(`languages.${l}`)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t("kind")}>
          {({ id }) => (
            <Select id={id} value={filters.kind ?? ""} onChange={(e) => update({ kind: e.target.value || undefined })}>
              <option value="">{tc("all")}</option>
              {KINDS.map((k) => (
                <option key={k} value={k}>
                  {t(`kinds.${k}`)}
                </option>
              ))}
            </Select>
          )}
        </Field>
      </div>

      {query.isPending ? <SkeletonRows rows={6} /> : null}
      {query.isError ? <ErrorNotice error={query.error} onRetry={() => void query.refetch()} /> : null}
      {query.data ? (
        query.data.items.length === 0 ? (
          <EmptyState
            title={t("empty")}
            description={t("emptyHint")}
            action={
              <Link href="/studio/new" className="inline-flex h-10 items-center rounded-[10px] border border-coral bg-coral px-4 font-display text-sm font-medium text-white hover:bg-coral-strong">
                {t("makeReels")}
              </Link>
            }
          />
        ) : (
          <Table>
            <thead>
              <tr>
                <Th>{t("reel")}</Th>
                <Th>{t("topic")}</Th>
                <Th>{t("kind")}</Th>
                <Th>{t("languages")}</Th>
                <Th>{t("status")}</Th>
                <Th>{t("recall")}</Th>
                <Th>{t("plays")}</Th>
                <Th>{t("updated")}</Th>
              </tr>
            </thead>
            <tbody>
              {query.data.items.map((r) => (
                <Tr key={r.id}>
                  <Td>
                    <Link href={`/studio/reels/${r.id}`} className="font-medium text-ink underline-offset-2 hover:underline">
                      {r.title}
                    </Link>
                    <p className="text-xs text-slate">
                      {formatDuration(r.duration_ms)} · v{r.version}
                    </p>
                  </Td>
                  <Td>{r.topic.name}</Td>
                  <Td>{t(`kinds.${r.kind}`)}</Td>
                  <Td>
                    <div className="flex flex-wrap gap-1">
                      {(r.languages ?? [r.language]).map((l) => (
                        <Badge key={l} tone="outline">
                          {l}
                        </Badge>
                      ))}
                    </div>
                  </Td>
                  <Td>
                    <div className="flex flex-col items-start gap-1">
                      <Badge tone={STATUS_TONE[r.status]}>{t(`statuses.${r.status}`)}</Badge>
                      {r.awaiting ? (
                        <Link href={`/studio/reels/${r.id}/publish`} className="text-xs text-slate underline underline-offset-2">
                          {t("awaiting", { what: r.awaiting })}
                        </Link>
                      ) : null}
                    </div>
                  </Td>
                  <Td>
                    <RecallBar value={r.recall_pct} />
                  </Td>
                  <Td className="tabular-nums">{formatNumber(r.plays, locale)}</Td>
                  <Td className="whitespace-nowrap text-slate">{formatDateTime(r.updated_at, locale)}</Td>
                </Tr>
              ))}
            </tbody>
          </Table>
        )
      ) : null}
    </>
  );
}
