"use client";

import Link from "next/link";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { useCreateTopic, useTopics } from "@/lib/api/hooks/topics";
import { usePeople } from "@/lib/api/hooks/people";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Field } from "@/components/ui/field";
import { Checkbox, Select } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { SkeletonRows } from "@/components/ui/skeleton";
import { useToast } from "@/components/ui/toast";
import { CsvImportDialog } from "@/features/people/csv-import-dialog";
import { InvitePersonDialog } from "@/features/people/invite-person-dialog";

const TEMPLATES: Record<string, { name: string; name_ar: string; compliance_tag?: string; is_mandatory: boolean }[]> = {
  retail: [
    { name: "Customer service", name_ar: "خدمة العملاء", is_mandatory: false },
    { name: "Store safety", name_ar: "سلامة المتجر", compliance_tag: "HRDF-SAF", is_mandatory: true },
    { name: "AML", name_ar: "مكافحة غسل الأموال", compliance_tag: "SAMA-AML", is_mandatory: true },
    { name: "Returns and refunds", name_ar: "الإرجاع والاسترداد", is_mandatory: false },
  ],
  logistics: [
    { name: "Warehouse safety", name_ar: "سلامة المستودع", compliance_tag: "HRDF-SAF", is_mandatory: true },
    { name: "Forklift basics", name_ar: "أساسيات الرافعة الشوكية", is_mandatory: true },
    { name: "Cold chain", name_ar: "سلسلة التبريد", is_mandatory: false },
  ],
  hospitality: [
    { name: "Guest welcome", name_ar: "استقبال الضيوف", is_mandatory: false },
    { name: "Food safety", name_ar: "سلامة الغذاء", compliance_tag: "SFDA-FS", is_mandatory: true },
    { name: "Fire safety", name_ar: "السلامة من الحريق", compliance_tag: "HRDF-SAF", is_mandatory: true },
  ],
  banking: [
    { name: "AML", name_ar: "مكافحة غسل الأموال", compliance_tag: "SAMA-AML", is_mandatory: true },
    { name: "KYC", name_ar: "اعرف عميلك", compliance_tag: "SAMA-KYC", is_mandatory: true },
    { name: "Data privacy", name_ar: "خصوصية البيانات", compliance_tag: "PDPL", is_mandatory: true },
  ],
};

export function TopicsStep() {
  const t = useTranslations("onboarding.topics");
  const router = useRouter();
  const { toast } = useToast();
  const topics = useTopics();
  const people = usePeople({});
  const create = useCreateTopic();
  const [sector, setSector] = useState("retail");
  const [picked, setPicked] = useState<string[]>([]);
  const [invite, setInvite] = useState(false);
  const [importOpen, setImportOpen] = useState(false);

  const templates = TEMPLATES[sector] ?? [];
  const existingNames = new Set((topics.data ?? []).map((x) => x.name.toLowerCase()));

  const addPicked = async () => {
    const toCreate = templates.filter((tp) => picked.includes(tp.name) && !existingNames.has(tp.name.toLowerCase()));
    for (const tp of toCreate) await create.mutateAsync(tp);
    toast({ title: t("added", { n: toCreate.length }), tone: "success" });
    setPicked([]);
  };

  return (
    <>
      <PageHeader eyebrow={t("step")} title={t("title")} description={t("subtitle")} />
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <Card>
          <CardHeader title={t("templates")} description={t("templatesHint")} />
          <CardBody className="flex flex-col gap-3">
            <Field label={t("sector")}>
              {({ id }) => (
                <Select id={id} value={sector} onChange={(e) => setSector(e.target.value)}>
                  {Object.keys(TEMPLATES).map((s) => (
                    <option key={s} value={s}>
                      {t(`sectors.${s}`)}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
            <div className="flex flex-col gap-2">
              {templates.map((tp) => {
                const exists = existingNames.has(tp.name.toLowerCase());
                return (
                  <Checkbox
                    key={tp.name}
                    label={
                      <span className="flex items-center gap-2">
                        {tp.name} <span className="text-slate" lang="ar">{tp.name_ar}</span>
                        {tp.compliance_tag ? <Badge tone="outline">{tp.compliance_tag}</Badge> : null}
                        {exists ? <Badge tone="mint">{t("alreadyAdded")}</Badge> : null}
                      </span>
                    }
                    disabled={exists}
                    checked={picked.includes(tp.name)}
                    onChange={(e) => setPicked((p) => (e.target.checked ? [...p, tp.name] : p.filter((x) => x !== tp.name)))}
                  />
                );
              })}
            </div>
            {create.error ? <ErrorNotice error={create.error} /> : null}
            <Button variant="secondary" disabled={picked.length === 0} loading={create.isPending} onClick={() => void addPicked()}>
              {t("addSelected", { n: picked.length })}
            </Button>
            <div className="text-sm text-slate">
              {topics.isPending ? <SkeletonRows rows={2} /> : topics.data ? t("youHave", { n: topics.data.length }) : null}{" "}
              <Link href="/topics" className="underline underline-offset-2">
                {t("manageTopics")}
              </Link>
            </div>
          </CardBody>
        </Card>
        <Card>
          <CardHeader title={t("people")} description={t("peopleHint")} />
          <CardBody className="flex flex-col gap-3">
            <div className="flex flex-wrap gap-2">
              <Button onClick={() => setImportOpen(true)}>{t("importCsv")}</Button>
              <Button onClick={() => setInvite(true)}>{t("inviteOne")}</Button>
              <Link href="/settings" className="inline-flex h-10 items-center rounded-[10px] border border-line bg-surface px-4 font-display text-sm font-medium text-ink hover:bg-surface-muted">
                {t("connectHris")}
              </Link>
            </div>
            <p className="text-sm text-slate">{people.data ? t("peopleCount", { n: people.data.items.length }) : null}</p>
            <p className="rounded-[10px] bg-paper px-3 py-2 text-xs text-slate">{t("privacy")}</p>
          </CardBody>
        </Card>
      </div>
      <div className="mt-6 flex justify-end gap-2">
        <Button variant="primary" onClick={() => router.push("/onboarding/delivery")}>
          {t("continue")}
        </Button>
      </div>
      <InvitePersonDialog open={invite} onClose={() => setInvite(false)} />
      <CsvImportDialog open={importOpen} onClose={() => setImportOpen(false)} />
    </>
  );
}
