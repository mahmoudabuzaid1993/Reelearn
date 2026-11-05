"use client";

import { useLocale, useTranslations } from "next-intl";
import { useState } from "react";
import { useAudit } from "@/lib/api/hooks/settings";
import { formatDateTime } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Select } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { Table, Td, Th, Tr } from "@/components/ui/table";

const ENTITIES = ["", "reel", "topic", "user", "generation_job", "integration", "tenant", "source"] as const;

export function AuditView() {
  const t = useTranslations("audit");
  const locale = useLocale();
  const [entity, setEntity] = useState<string>("");
  const query = useAudit({ entity: entity || undefined });

  return (
    <>
      <PageHeader title={t("title")} description={t("subtitle")} />
      <div className="mb-4 max-w-xs">
        <Field label={t("entity")}>
          {({ id }) => (
            <Select id={id} value={entity} onChange={(e) => setEntity(e.target.value)}>
              {ENTITIES.map((e) => (
                <option key={e} value={e}>
                  {e ? t(`entities.${e}`) : t("allEntities")}
                </option>
              ))}
            </Select>
          )}
        </Field>
      </div>
      {query.isPending ? <SkeletonRows rows={8} /> : null}
      {query.isError ? <ErrorNotice error={query.error} onRetry={() => void query.refetch()} /> : null}
      {query.data ? (
        query.data.items.length === 0 ? (
          <EmptyState title={t("empty")} />
        ) : (
          <Table>
            <thead>
              <tr>
                <Th>{t("when")}</Th>
                <Th>{t("actor")}</Th>
                <Th>{t("action")}</Th>
                <Th>{t("entity")}</Th>
                <Th>{t("diff")}</Th>
              </tr>
            </thead>
            <tbody>
              {query.data.items.map((e) => (
                <Tr key={e.id}>
                  <Td className="whitespace-nowrap text-slate">{formatDateTime(e.at, locale)}</Td>
                  <Td>{e.actor?.full_name ?? <span className="text-slate">{t("system")}</span>}</Td>
                  <Td>
                    <code className="text-xs">{e.action}</code>
                  </Td>
                  <Td>
                    <Badge tone="outline">{e.entity}</Badge>
                  </Td>
                  <Td>
                    <code dir="ltr" className="block max-w-md truncate text-xs text-slate">
                      {e.diff ? JSON.stringify(e.diff) : "—"}
                    </code>
                  </Td>
                </Tr>
              ))}
            </tbody>
          </Table>
        )
      ) : null}
    </>
  );
}
