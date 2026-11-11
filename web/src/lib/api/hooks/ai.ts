"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../client";
import { unwrap } from "../errors";
import type { AiPlan, AiRouting } from "../types";
import { keys } from "@/lib/query/keys";

/** Which provider writes, the quality check, and whose key each provider uses. */
export function useAi() {
  return useQuery({ queryKey: keys.ai, queryFn: async () => unwrap(await api.GET("/admin/ai")) });
}

export function useSaveAiRouting() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: AiRouting) => unwrap(await api.PUT("/admin/ai", { body })),
    onSuccess: (data) => qc.setQueryData(keys.ai, data),
  });
}

/** `month` is `YYYY-MM`. */
export function useAiUsage(month: string) {
  return useQuery({
    queryKey: keys.aiUsage(month),
    queryFn: async () => unwrap(await api.GET("/admin/ai/usage", { params: { query: { month } } })),
  });
}

/** Whether the signed-in person runs the platform (shows the Platform page). */
export function usePlatformMe() {
  return useQuery({
    queryKey: keys.platformMe,
    queryFn: async () => unwrap(await api.GET("/platform/me")),
    staleTime: 5 * 60_000,
  });
}

export function usePlatformIntegrations() {
  return useQuery({ queryKey: keys.platformIntegrations, queryFn: async () => unwrap(await api.GET("/platform/integrations")) });
}

export function useSavePlatformIntegration() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ kind, body }: { kind: string; body: Record<string, unknown> }) =>
      unwrap(await api.PUT("/platform/integrations/{kind}", { params: { path: { kind: kind as never } }, body })),
    onSuccess: (data) => {
      qc.setQueryData(keys.platformIntegrations, data);
      void qc.invalidateQueries({ queryKey: keys.ai });
    },
  });
}

export function useTestPlatformIntegration() {
  return useMutation({
    mutationFn: async (kind: string) => unwrap(await api.POST("/platform/integrations/{kind}/test", { params: { path: { kind } } })),
  });
}

export function usePlatformTenants(month: string) {
  return useQuery({
    queryKey: keys.platformTenants(month),
    queryFn: async () => unwrap(await api.GET("/platform/tenants", { params: { query: { month } } })),
  });
}

export function useSaveAiPlan() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ tenantId, plan }: { tenantId: string; plan: AiPlan }) =>
      unwrap(await api.PUT("/platform/tenants/{tenant_id}/ai-plan", { params: { path: { tenant_id: tenantId } }, body: plan })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["platform-tenants"] });
      void qc.invalidateQueries({ queryKey: keys.ai });
    },
  });
}

/** The month's invoice lines as a CSV file. */
export async function downloadUsageCsv(month: string): Promise<Blob> {
  const { data, error } = await api.GET("/platform/usage.csv", { params: { query: { month } }, parseAs: "blob" });
  if (error || !data) throw error ?? new Error("download failed");
  return data as Blob;
}
