"use client";

import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../client";
import { unwrap } from "../errors";
import { keys } from "@/lib/query/keys";
import type { AssignmentWrite, TopicCreate } from "../types";

export function useTopics() {
  return useQuery({ queryKey: keys.topics, queryFn: async () => unwrap(await api.GET("/admin/topics")) });
}

export function useAssignments(topicId: string) {
  return useQuery({
    queryKey: keys.assignments(topicId),
    queryFn: async () => unwrap(await api.GET("/admin/topics/{topic_id}/assignments", { params: { path: { topic_id: topicId } } })),
    enabled: Boolean(topicId),
  });
}

export function useAllAssignments(topicIds: string[]) {
  return useQueries({
    queries: topicIds.map((topicId) => ({
      queryKey: keys.assignments(topicId),
      queryFn: async () => unwrap(await api.GET("/admin/topics/{topic_id}/assignments", { params: { path: { topic_id: topicId } } })),
    })),
  });
}

export function useSaveAssignments() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ topicId, body }: { topicId: string; body: AssignmentWrite[] }) =>
      unwrap(await api.PUT("/admin/topics/{topic_id}/assignments", { params: { path: { topic_id: topicId } }, body })),
    onSuccess: (data, vars) => {
      qc.setQueryData(keys.assignments(vars.topicId), data);
      void qc.invalidateQueries({ queryKey: keys.topics });
    },
  });
}

export function useCreateTopic() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: TopicCreate) => unwrap(await api.POST("/admin/topics", { body })),
    onSuccess: () => void qc.invalidateQueries({ queryKey: keys.topics }),
  });
}

export function useUpdateTopic() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, body }: { id: string; body: TopicCreate }) =>
      unwrap(await api.PATCH("/admin/topics/{topic_id}", { params: { path: { topic_id: id } }, body })),
    onSuccess: () => void qc.invalidateQueries({ queryKey: keys.topics }),
  });
}
