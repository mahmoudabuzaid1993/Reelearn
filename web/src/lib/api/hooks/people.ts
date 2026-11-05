"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, API_BASE, authHeaders } from "../client";
import { ApiError, asPage, unwrap } from "../errors";
import { keys } from "@/lib/query/keys";
import type { ImportReport, UserCreate, Schemas } from "../types";

export interface PeopleFilters {
  q?: string;
  role?: Schemas["Role"];
  status?: Schemas["UserStatus"];
  group_id?: string;
  language?: string;
}

export function usePeople(filters: PeopleFilters = {}) {
  return useQuery({
    queryKey: keys.users(filters as Record<string, string | undefined>),
    queryFn: async () => asPage(unwrap(await api.GET("/admin/users", { params: { query: { ...filters, limit: 100 } } }))),
  });
}

export function useGroups() {
  return useQuery({ queryKey: keys.groups, queryFn: async () => unwrap(await api.GET("/admin/groups")) });
}

export function useInvitePerson() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: UserCreate) => unwrap(await api.POST("/admin/users", { body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["users"] });
      void qc.invalidateQueries({ queryKey: keys.groups });
    },
  });
}

export function useUpdatePerson() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, body }: { id: string; body: Schemas["UserPatch"] }) =>
      unwrap(await api.PATCH("/admin/users/{user_id}", { params: { path: { user_id: id } }, body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["users"] });
      void qc.invalidateQueries({ queryKey: keys.groups });
    },
  });
}

export function useCreateGroup() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: { name: string; kind?: "team" | "location" | "role" | "custom" }) =>
      unwrap(await api.POST("/admin/groups", { body })),
    onSuccess: () => void qc.invalidateQueries({ queryKey: keys.groups }),
  });
}

/** text/csv body: openapi-fetch would JSON-encode a string, so this goes through fetch with the same auth header. */
export function useImportCsv() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ csv, dryRun }: { csv: string; dryRun: boolean }): Promise<ImportReport> => {
      const res = await fetch(`${API_BASE}/admin/users/import?dry_run=${dryRun}`, {
        method: "POST",
        headers: { ...authHeaders(), "content-type": "text/csv" },
        body: csv,
      });
      if (!res.ok) throw new ApiError(res.status, await res.json().catch(() => null));
      return (await res.json()) as ImportReport;
    },
    onSuccess: (_report, vars) => {
      if (!vars.dryRun) void qc.invalidateQueries({ queryKey: ["users"] });
    },
  });
}

export function useHrisSyncLog() {
  return useQuery({
    queryKey: ["hris-sync-log"],
    queryFn: async () => {
      const res = await fetch(`${API_BASE}/admin/integrations/hris/sync-log`, { headers: authHeaders() });
      if (!res.ok) throw new ApiError(res.status, await res.json().catch(() => null));
      return (await res.json()) as { at: string; provider: string; created: number; updated: number; skipped: number; status: string; message?: string }[];
    },
  });
}
