"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../client";
import { unwrap } from "../errors";
import { keys } from "@/lib/query/keys";

export type Range = "7d" | "30d" | "90d" | "12m";

export function useReportOverview(range: Range) {
  return useQuery({ queryKey: keys.reports("overview", range), queryFn: async () => unwrap(await api.GET("/admin/reports/overview", { params: { query: { range } } })) });
}
export function useReportByTopic(range: Range) {
  return useQuery({ queryKey: keys.reports("topic", range), queryFn: async () => unwrap(await api.GET("/admin/reports/by-topic", { params: { query: { range } } })) });
}
export function useReportByGroup(range: Range) {
  return useQuery({ queryKey: keys.reports("group", range), queryFn: async () => unwrap(await api.GET("/admin/reports/by-group", { params: { query: { range } } })) });
}
export function useExports(refetchInterval: number | false = false) {
  return useQuery({ queryKey: keys.exports, queryFn: async () => unwrap(await api.GET("/admin/exports")), refetchInterval });
}
export function useCreateExport() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: { kind: "hrdf_evidence"; period_start: string; period_end: string }) => unwrap(await api.POST("/admin/exports", { body })),
    onSuccess: () => void qc.invalidateQueries({ queryKey: keys.exports }),
  });
}
export function useDownloadExport() {
  return useMutation({
    mutationFn: async (id: string) => unwrap(await api.GET("/admin/exports/{export_id}/download", { params: { path: { export_id: id } } })),
  });
}
