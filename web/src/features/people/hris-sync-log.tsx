"use client";

import Link from "next/link";
import { useLocale, useTranslations } from "next-intl";
import { useHrisSyncLog } from "@/lib/api/hooks/people";
import { formatDateTime } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { SkeletonRows } from "@/components/ui/skeleton";
import { Table, Td, Th, Tr } from "@/components/ui/table";

export function HrisSyncLog() {
  const t = useTranslations("people.hris");
  const locale = useLocale();
  const query = useHrisSyncLog();
  return (
    <Card>
      <CardHeader
        title={t("title")}
        description={t("hint")}
        actions={
          <Link href="/settings" className="text-sm font-medium text-ink underline underline-offset-2">
            {t("manage")}
          </Link>
        }
      />
      {query.isPending ? (
        <div className="p-5">
          <SkeletonRows rows={4} />
        </div>
      ) : query.isError ? (
        <div className="p-5">
          <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />
        </div>
      ) : query.data.length === 0 ? (
        <div className="p-5">
          <EmptyState title={t("empty")} description={t("emptyHint")} />
        </div>
      ) : (
        <Table className="rounded-none border-0">
          <thead>
            <tr>
              <Th>{t("when")}</Th>
              <Th>{t("provider")}</Th>
              <Th>{t("created")}</Th>
              <Th>{t("updated")}</Th>
              <Th>{t("skipped")}</Th>
              <Th>{t("status")}</Th>
            </tr>
          </thead>
          <tbody>
            {query.data.map((row) => (
              <Tr key={row.at}>
                <Td className="whitespace-nowrap text-slate">{formatDateTime(row.at, locale)}</Td>
                <Td>{row.provider}</Td>
                <Td>{row.created}</Td>
                <Td>{row.updated}</Td>
                <Td>{row.skipped}</Td>
                <Td>
                  <Badge tone={row.status === "ok" ? "mint" : "coral"}>{row.status === "ok" ? t("ok") : row.message ?? t("error")}</Badge>
                </Td>
              </Tr>
            ))}
          </tbody>
        </Table>
      )}
    </Card>
  );
}
