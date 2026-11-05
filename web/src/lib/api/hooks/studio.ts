"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, API_BASE, authHeaders } from "../client";
import { ApiError, asPage, unwrap } from "../errors";
import { keys } from "@/lib/query/keys";
import type { GenerationRequest, SourceCreate } from "../types";

export function useSources() {
  return useQuery({ queryKey: keys.sources, queryFn: async () => asPage(unwrap(await api.GET("/admin/sources", { params: { query: { limit: 100 } } }))) });
}

export function useSource(id: string) {
  return useQuery({
    queryKey: keys.source(id),
    queryFn: async () => unwrap(await api.GET("/admin/sources/{source_id}", { params: { path: { source_id: id } } })),
    enabled: Boolean(id),
  });
}

export function useCreateSource() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: SourceCreate) => unwrap(await api.POST("/admin/sources", { body })),
    onSuccess: () => void qc.invalidateQueries({ queryKey: keys.sources }),
  });
}

export function useRequestUpload() {
  return useMutation({
    mutationFn: async (body: { filename: string; mime: string; bytes: number }) => unwrap(await api.POST("/admin/uploads", { body })),
  });
}

export function useMoments(sourceId: string) {
  return useQuery({
    queryKey: keys.moments(sourceId),
    queryFn: async () => unwrap(await api.GET("/admin/sources/{source_id}/moments", { params: { path: { source_id: sourceId } } })),
    enabled: Boolean(sourceId),
  });
}

export interface TranscriptLine {
  start_ms: number;
  end_ms: number;
  speaker: string;
  text: string;
}

/** Transcript lines are not yet in the contract; this fetch is isolated so swapping it is one line. */
export function useTranscript(sourceId: string) {
  return useQuery({
    queryKey: ["transcript", sourceId],
    queryFn: async (): Promise<TranscriptLine[]> => {
      const res = await fetch(`${API_BASE}/admin/sources/${sourceId}/transcript`, { headers: authHeaders() });
      if (!res.ok) throw new ApiError(res.status, await res.json().catch(() => null));
      return (await res.json()) as TranscriptLine[];
    },
    enabled: Boolean(sourceId),
  });
}

export function useCutMoment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ sourceId, momentId, body }: { sourceId: string; momentId: string; body: { topic_id: string; start_ms?: number; end_ms?: number; title?: string; speaker_consent: boolean } }) =>
      unwrap(await api.POST("/admin/sources/{source_id}/moments/{moment_id}/cut", { params: { path: { source_id: sourceId, moment_id: momentId } }, body })),
    onSuccess: (_reel, vars) => {
      void qc.invalidateQueries({ queryKey: keys.moments(vars.sourceId) });
      void qc.invalidateQueries({ queryKey: ["reels"] });
    },
  });
}

export function useQuote(request: GenerationRequest | null) {
  return useQuery({
    queryKey: ["quote", request],
    queryFn: async () => unwrap(await api.POST("/admin/generation-jobs/quote", { body: request! })),
    enabled: Boolean(request && request.source_id && request.topic_id && request.languages.length > 0),
    staleTime: 0,
  });
}

export function useStartJob() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: GenerationRequest) => unwrap(await api.POST("/admin/generation-jobs", { body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: keys.jobs });
      void qc.invalidateQueries({ queryKey: keys.tenant });
    },
  });
}

export function useJobs() {
  return useQuery({ queryKey: keys.jobs, queryFn: async () => asPage(unwrap(await api.GET("/admin/generation-jobs", { params: { query: { limit: 50 } } }))) });
}

export function useJob(id: string, refetchInterval: number | false = false) {
  return useQuery({
    queryKey: keys.job(id),
    queryFn: async () => unwrap(await api.GET("/admin/generation-jobs/{job_id}", { params: { path: { job_id: id } } })),
    enabled: Boolean(id),
    refetchInterval,
  });
}

export function useRetryJob() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => unwrap(await api.POST("/admin/generation-jobs/{job_id}/retry", { params: { path: { job_id: id } } })),
    onSuccess: (job) => qc.setQueryData(keys.job(job.id), job),
  });
}

export function useStoryboard(jobId: string) {
  return useQuery({
    queryKey: keys.storyboard(jobId),
    queryFn: async () => unwrap(await api.GET("/admin/generation-jobs/{job_id}/storyboard", { params: { path: { job_id: jobId } } })),
    enabled: Boolean(jobId),
  });
}
