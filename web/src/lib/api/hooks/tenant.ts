"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../client";
import { unwrap } from "../errors";
import { keys } from "@/lib/query/keys";
import type { Paths, Schemas } from "../types";

type TenantCreateBody = Paths["/tenants"]["post"]["requestBody"]["content"]["application/json"];

export function useTenant() {
  return useQuery({ queryKey: keys.tenant, queryFn: async () => unwrap(await api.GET("/admin/tenant")) });
}

export function useUpdateTenant() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: Schemas["TenantPatch"]) => unwrap(await api.PATCH("/admin/tenant", { body })),
    onSuccess: (tenant) => qc.setQueryData(keys.tenant, tenant),
  });
}

export function useDashboard(range: "7d" | "30d" | "90d" | "12m" = "30d") {
  return useQuery({
    queryKey: keys.dashboard(range),
    queryFn: async () => unwrap(await api.GET("/admin/dashboard", { params: { query: { range } } })),
  });
}

export function useCreateTenant() {
  return useMutation({
    mutationFn: async (body: TenantCreateBody) => unwrap(await api.POST("/tenants", { body })),
  });
}
