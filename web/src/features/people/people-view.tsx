"use client";

import { useLocale, useTranslations } from "next-intl";
import { useRouter, useSearchParams } from "next/navigation";
import { useState } from "react";
import { useGroups, usePeople, type PeopleFilters } from "@/lib/api/hooks/people";
import type { Role, UserStatus } from "@/lib/api/types";
import { formatDateTime } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Input, Select } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { Table, Td, Th, Tr } from "@/components/ui/table";
import { Tabs } from "@/components/ui/tabs";
import { CsvImportDialog } from "./csv-import-dialog";
import { GroupsPanel } from "./groups-panel";
import { HrisSyncLog } from "./hris-sync-log";
import { InvitePersonDialog } from "./invite-person-dialog";

type Tab = "people" | "groups" | "hris";

export function PeopleView() {
  const t = useTranslations("people");
  const params = useSearchParams();
  const router = useRouter();
  const [tab, setTab] = useState<Tab>((params.get("tab") as Tab) || "people");
  const [invite, setInvite] = useState(false);
  const [importOpen, setImportOpen] = useState(false);

  return (
    <>
      <PageHeader
        title={t("title")}
        description={t("subtitle")}
        actions={
          <>
            <Button onClick={() => setImportOpen(true)}>{t("importCsv")}</Button>
            <Button variant="primary" onClick={() => setInvite(true)}>
              {t("invite")}
            </Button>
          </>
        }
      />
      <Tabs
        ariaLabel={t("title")}
        value={tab}
        onChange={(v) => {
          setTab(v);
          router.replace(`/people?tab=${v}`);
        }}
        items={[{ value: "people", label: t("tabs.people") }, { value: "groups", label: t("tabs.groups") }, { value: "hris", label: t("tabs.hris") }]}
      />
      <div className="mt-6">
        {tab === "people" ? <PeopleTable initialQuery={params.get("q") ?? ""} /> : null}
        {tab === "groups" ? <GroupsPanel /> : null}
        {tab === "hris" ? <HrisSyncLog /> : null}
      </div>
      <InvitePersonDialog open={invite} onClose={() => setInvite(false)} />
      <CsvImportDialog open={importOpen} onClose={() => setImportOpen(false)} />
    </>
  );
}

const STATUS_TONE: Record<UserStatus, "mint" | "neutral" | "ink" | "outline"> = { active: "mint", invited: "ink", paused: "neutral", deactivated: "outline" };

function PeopleTable({ initialQuery }: { initialQuery: string }) {
  const t = useTranslations("people");
  const tc = useTranslations("common");
  const locale = useLocale();
  const [filters, setFilters] = useState<PeopleFilters>({ q: initialQuery || undefined });
  const query = usePeople(filters);
  const groups = useGroups();

  return (
    <div className="flex flex-col gap-4">
      <div className="grid grid-cols-2 gap-3 md:grid-cols-5">
        <Field label={t("search")} className="col-span-2 md:col-span-1">
          {({ id }) => <Input id={id} type="search" value={filters.q ?? ""} onChange={(e) => setFilters((f) => ({ ...f, q: e.target.value || undefined }))} placeholder={t("searchPlaceholder")} />}
        </Field>
        <Field label={t("role")}>
          {({ id }) => (
            <Select id={id} value={filters.role ?? ""} onChange={(e) => setFilters((f) => ({ ...f, role: (e.target.value || undefined) as Role | undefined }))}>
              <option value="">{tc("all")}</option>
              {(["admin", "manager", "sme", "learner"] as Role[]).map((r) => (
                <option key={r} value={r}>
                  {tc(`roles.${r}`)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t("status")}>
          {({ id }) => (
            <Select id={id} value={filters.status ?? ""} onChange={(e) => setFilters((f) => ({ ...f, status: (e.target.value || undefined) as UserStatus | undefined }))}>
              <option value="">{tc("all")}</option>
              {(["invited", "active", "paused", "deactivated"] as UserStatus[]).map((s) => (
                <option key={s} value={s}>
                  {t(`statuses.${s}`)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t("group")}>
          {({ id }) => (
            <Select id={id} value={filters.group_id ?? ""} onChange={(e) => setFilters((f) => ({ ...f, group_id: e.target.value || undefined }))}>
              <option value="">{tc("all")}</option>
              {(groups.data ?? []).map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t("language")}>
          {({ id }) => (
            <Select id={id} value={filters.language ?? ""} onChange={(e) => setFilters((f) => ({ ...f, language: e.target.value || undefined }))}>
              <option value="">{tc("all")}</option>
              {["ar", "en", "ur", "bn", "hi", "tl"].map((l) => (
                <option key={l} value={l}>
                  {tc(`languages.${l}`)}
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
          <EmptyState title={t("empty")} description={t("emptyHint")} />
        ) : (
          <Table>
            <thead>
              <tr>
                <Th>{t("name")}</Th>
                <Th>{t("role")}</Th>
                <Th>{t("language")}</Th>
                <Th>{t("channel")}</Th>
                <Th>{t("groups")}</Th>
                <Th>{t("status")}</Th>
                <Th>{t("lastSeen")}</Th>
              </tr>
            </thead>
            <tbody>
              {query.data.items.map((u) => (
                <Tr key={u.id}>
                  <Td>
                    <p className="font-medium text-ink">{u.full_name}</p>
                    <p className="text-xs text-slate" dir="ltr">
                      {u.email ?? u.phone_e164 ?? u.employee_no}
                    </p>
                  </Td>
                  <Td>{tc(`roles.${u.role}`)}</Td>
                  <Td>{tc(`languages.${u.preferred_language}`)}</Td>
                  <Td>{tc(`channels.${u.delivery_channel}`)}</Td>
                  <Td>
                    <div className="flex flex-wrap gap-1">
                      {u.groups.map((g) => (
                        <Badge key={g.id} tone="outline">
                          {g.name}
                        </Badge>
                      ))}
                    </div>
                  </Td>
                  <Td>
                    <Badge tone={STATUS_TONE[u.status]}>{t(`statuses.${u.status}`)}</Badge>
                  </Td>
                  <Td className="whitespace-nowrap text-slate">{u.last_seen_at ? formatDateTime(u.last_seen_at, locale) : t("never")}</Td>
                </Tr>
              ))}
            </tbody>
          </Table>
        )
      ) : null}
    </div>
  );
}
