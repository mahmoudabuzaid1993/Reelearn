"use client";

import { useTranslations } from "next-intl";
import { useMemo, useState } from "react";
import { useImportCsv, usePeople } from "@/lib/api/hooks/people";
import type { ImportReport } from "@/lib/api/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Textarea } from "@/components/ui/input";
import { Table, Td, Th, Tr } from "@/components/ui/table";
import { useToast } from "@/components/ui/toast";
import { CSV_COLUMNS, previewImport } from "./logic/csv";

const SAMPLE = `full_name,role,phone,email,language,location,groups
Layla Hassan,learner,0551234567,,ar,Store 12 · Jeddah,Store 12 · Jeddah
Omar Siddiqui,learner,0559876543,,ur,Jeddah warehouse,Jeddah warehouse`;

export function CsvImportDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const t = useTranslations("people.import");
  const tc = useTranslations("common");
  const { toast } = useToast();
  const [csv, setCsv] = useState("");
  const [serverReport, setServerReport] = useState<ImportReport | null>(null);
  const people = usePeople({});
  const importCsv = useImportCsv();

  const preview = useMemo(
    () => (csv.trim() ? previewImport(csv, (people.data?.items ?? []).map((u) => ({ email: u.email, phone: u.phone_e164, employee_no: u.employee_no }))) : null),
    [csv, people.data],
  );

  const onFile = async (file: File | undefined) => {
    if (!file) return;
    setCsv(await file.text());
    setServerReport(null);
  };

  const close = () => {
    setCsv("");
    setServerReport(null);
    onClose();
  };

  return (
    <Dialog
      open={open}
      onClose={close}
      size="lg"
      title={t("title")}
      description={t("hint")}
      footer={
        <>
          <Button onClick={close}>{tc("cancel")}</Button>
          <Button
            disabled={!preview || preview.totals.total === 0}
            loading={importCsv.isPending && importCsv.variables?.dryRun === true}
            onClick={() => importCsv.mutate({ csv, dryRun: true }, { onSuccess: setServerReport })}
          >
            {t("dryRun")}
          </Button>
          <Button
            variant="primary"
            disabled={!serverReport || !serverReport.dry_run || (serverReport.created ?? 0) + (serverReport.updated ?? 0) === 0}
            loading={importCsv.isPending && importCsv.variables?.dryRun === false}
            onClick={() =>
              importCsv.mutate(
                { csv, dryRun: false },
                {
                  onSuccess: (r) => {
                    toast({ title: t("done", { created: r.created ?? 0, updated: r.updated ?? 0 }), tone: "success" });
                    close();
                  },
                },
              )
            }
          >
            {t("import")}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <div className="flex flex-wrap items-end gap-3">
          <Field label={t("file")} className="flex-1">
            {({ id }) => <input id={id} type="file" accept=".csv,text/csv" className="block w-full text-sm text-slate file:me-3 file:rounded-md file:border file:border-line file:bg-surface file:px-3 file:py-1.5 file:text-sm file:text-ink" onChange={(e) => void onFile(e.target.files?.[0])} />}
          </Field>
          <Button size="sm" variant="quiet" onClick={() => setCsv(SAMPLE)}>
            {t("useSample")}
          </Button>
        </div>
        <Field label={t("paste")} hint={t("columns", { columns: CSV_COLUMNS.join(", ") })}>
          {({ id, describedBy }) => (
            <Textarea
              id={id}
              dir="ltr"
              aria-describedby={describedBy}
              className="font-mono text-xs"
              value={csv}
              onChange={(e) => {
                setCsv(e.target.value);
                setServerReport(null);
              }}
            />
          )}
        </Field>

        {preview ? (
          <section aria-label={t("previewTitle")} className="flex flex-col gap-3">
            <div className="flex flex-wrap items-center gap-2 text-sm">
              <Badge tone="ink">{t("totals.total", { n: preview.totals.total })}</Badge>
              <Badge tone="mint">{t("totals.create", { n: preview.totals.create })}</Badge>
              <Badge tone="neutral">{t("totals.update", { n: preview.totals.update })}</Badge>
              <Badge tone="outline">{t("totals.skip", { n: preview.totals.skip })}</Badge>
              {preview.unmappedHeaders.length > 0 ? <span className="text-xs text-slate">{t("unmapped", { headers: preview.unmappedHeaders.join(", ") })}</span> : null}
            </div>
            <div className="max-h-64 overflow-auto">
              <Table>
                <thead>
                  <tr>
                    <Th>#</Th>
                    <Th>{t("colName")}</Th>
                    <Th>{t("colContact")}</Th>
                    <Th>{t("colAction")}</Th>
                    <Th>{t("colIssues")}</Th>
                  </tr>
                </thead>
                <tbody>
                  {preview.rows.map((r) => (
                    <Tr key={r.row}>
                      <Td className="text-slate">{r.row}</Td>
                      <Td>{r.values.full_name ?? <span className="text-slate">—</span>}</Td>
                      <Td dir="ltr">{r.values.email ?? r.values.phone ?? "—"}</Td>
                      <Td>
                        <Badge tone={r.action === "create" ? "mint" : r.action === "update" ? "neutral" : "outline"}>{t(`actions.${r.action}`)}</Badge>
                      </Td>
                      <Td className="text-xs text-slate">{r.errors.map((e) => t(`errors.${e}`)).join(", ")}</Td>
                    </Tr>
                  ))}
                </tbody>
              </Table>
            </div>
          </section>
        ) : null}

        {importCsv.error ? <ErrorNotice error={importCsv.error} /> : null}
        {serverReport ? (
          <div role="status" className="rounded-[10px] border border-mint bg-mint-soft px-4 py-3 text-sm text-ink">
            <p className="font-medium">{serverReport.dry_run ? t("serverDryRun") : t("serverDone")}</p>
            <p className="text-xs text-slate">
              {t("serverSummary", { total: serverReport.total_rows ?? 0, created: serverReport.created ?? 0, updated: serverReport.updated ?? 0, skipped: serverReport.skipped ?? 0 })}
            </p>
            {serverReport.errors?.length ? (
              <ul className="mt-1 list-inside list-disc text-xs text-slate">
                {serverReport.errors.map((e) => (
                  <li key={e.row}>
                    {t("rowError", { row: e.row ?? 0, message: e.message ?? "" })}
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
        ) : null}
      </div>
    </Dialog>
  );
}
