"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../client";
import { asPage, unwrap } from "../errors";
import { keys } from "@/lib/query/keys";
import type { IntegrationKind } from "../types";

export function useIntegrations() {
  return useQuery({ queryKey: keys.integrations, queryFn: async () => unwrap(await api.GET("/admin/integrations")) });
}

export function useSaveIntegration() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ kind, body }: { kind: IntegrationKind; body: Record<string, unknown> }) =>
      unwrap(await api.PUT("/admin/integrations/{kind}", { params: { path: { kind } }, body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: keys.integrations });
      // A company key changes which key each AI provider resolves to.
      void qc.invalidateQueries({ queryKey: keys.ai });
    },
  });
}

export function useTestIntegration() {
  return useMutation({
    mutationFn: async (kind: IntegrationKind) => unwrap(await api.POST("/admin/integrations/{kind}/test", { params: { path: { kind } } })),
  });
}

export function useAudit(filters: { entity?: string; entity_id?: string }) {
  return useQuery({
    queryKey: keys.audit(filters),
    queryFn: async () => asPage(unwrap(await api.GET("/admin/audit", { params: { query: { ...filters, limit: 100 } } }))),
  });
}
