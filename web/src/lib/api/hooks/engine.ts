"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../client";
import { unwrap } from "../errors";
import type { EngineSettings, Intensity } from "../types";
import { keys } from "@/lib/query/keys";
import type { Range } from "./reports";

export function useEngine() {
  return useQuery({ queryKey: keys.engine, queryFn: async () => unwrap(await api.GET("/admin/engine")) });
}

/** A preset name alone switches preset; anything else is merged and re-labelled by the API. */
export type EnginePatch = { intensity: Exclude<Intensity, "custom"> } | Partial<EngineSettings>;

export function useSaveEngine() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: EnginePatch) => unwrap(await api.PUT("/admin/engine", { body: body as never })),
    onSuccess: (data) => {
      qc.setQueryData(keys.engine, data);
      void qc.invalidateQueries({ queryKey: ["engine-insights"] });
    },
  });
}

export function useEngineInsights(range: Range) {
  return useQuery({
    queryKey: keys.engineInsights(range),
    queryFn: async () => unwrap(await api.GET("/admin/engine/insights", { params: { query: { range } } })),
  });
}
