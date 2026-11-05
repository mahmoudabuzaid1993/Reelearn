"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../client";
import { asPage, unwrap } from "../errors";
import { keys } from "@/lib/query/keys";
import type { LibraryFilters } from "@/features/library/logic/filters";
import type { Reel, ReelPatch, ReelKind, ReelStatus } from "../types";

export function useReels(filters: LibraryFilters) {
  return useQuery({
    queryKey: keys.reels(filters as Record<string, string | undefined>),
    queryFn: async () =>
      asPage(unwrap(
        await api.GET("/admin/reels", {
          params: {
            query: {
              status: filters.status as ReelStatus | undefined,
              topic_id: filters.topic_id,
              language: filters.language,
              kind: filters.kind as ReelKind | undefined,
              q: filters.q,
              limit: 100,
            },
          },
        }),
      )),
  });
}

export function useReel(id: string) {
  return useQuery({
    queryKey: keys.reel(id),
    queryFn: async () => unwrap(await api.GET("/admin/reels/{reel_id}", { params: { path: { reel_id: id } } })),
    enabled: Boolean(id),
  });
}

function useReelMutation<TVars>(fn: (vars: TVars) => Promise<Reel>) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: (reel) => {
      qc.setQueryData(keys.reel(reel.id), reel);
      void qc.invalidateQueries({ queryKey: ["reels"] });
      void qc.invalidateQueries({ queryKey: ["storyboard"] });
      void qc.invalidateQueries({ queryKey: keys.topics });
    },
  });
}

export function useUpdateReel() {
  return useReelMutation(async ({ id, body }: { id: string; body: ReelPatch }) =>
    unwrap(await api.PATCH("/admin/reels/{reel_id}", { params: { path: { reel_id: id } }, body })),
  );
}

export function useSubmitReview() {
  return useReelMutation(async ({ id, smeUserId }: { id: string; smeUserId?: string }) =>
    unwrap(await api.POST("/admin/reels/{reel_id}/submit-review", { params: { path: { reel_id: id } }, body: { sme_user_id: smeUserId } })),
  );
}

export function useRecordApproval() {
  return useReelMutation(async ({ id, decision, comment }: { id: string; decision: "approved" | "changes_requested" | "rejected"; comment?: string }) =>
    unwrap(await api.POST("/admin/reels/{reel_id}/approvals", { params: { path: { reel_id: id } }, body: { decision, comment } })),
  );
}

export function usePublishReel() {
  return useReelMutation(async ({ id, group_ids, starts_on, compliance_tag }: { id: string; group_ids: string[]; starts_on?: string; compliance_tag?: string }) =>
    unwrap(await api.POST("/admin/reels/{reel_id}/publish", { params: { path: { reel_id: id } }, body: { group_ids, starts_on, compliance_tag } })),
  );
}

export function useArchiveReel() {
  return useReelMutation(async (id: string) => unwrap(await api.POST("/admin/reels/{reel_id}/archive", { params: { path: { reel_id: id } } })));
}

export function useRegenerateReel() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, what }: { id: string; what: ("visual" | "voice" | "script" | "question")[] }) =>
      unwrap(await api.POST("/admin/reels/{reel_id}/regenerate", { params: { path: { reel_id: id } }, body: { what } })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: keys.jobs });
      void qc.invalidateQueries({ queryKey: keys.tenant });
    },
  });
}

export function useAddVariants() {
  return useMutation({
    mutationFn: async ({ id, languages }: { id: string; languages: string[] }) =>
      unwrap(await api.POST("/admin/reels/{reel_id}/variants", { params: { path: { reel_id: id } }, body: { languages } })),
  });
}
