import type { ReelListItem } from "@/lib/api/types";

export interface LibraryFilters {
  status?: string;
  topic_id?: string;
  language?: string;
  kind?: string;
  q?: string;
}

export function applyLibraryFilters(items: ReelListItem[], filters: LibraryFilters): ReelListItem[] {
  const q = filters.q?.trim().toLowerCase();
  return items.filter((item) => {
    if (filters.status && item.status !== filters.status) return false;
    if (filters.topic_id && item.topic.id !== filters.topic_id) return false;
    if (filters.language && !(item.languages ?? [item.language]).includes(filters.language)) return false;
    if (filters.kind && item.kind !== filters.kind) return false;
    if (q && !item.title.toLowerCase().includes(q) && !(item.topic.name ?? "").toLowerCase().includes(q)) return false;
    return true;
  });
}

export function parseFilters(params: URLSearchParams): LibraryFilters {
  const pick = (k: string) => params.get(k) || undefined;
  return { status: pick("status"), topic_id: pick("topic_id"), language: pick("language"), kind: pick("kind"), q: pick("q") };
}

export function toSearchParams(filters: LibraryFilters): URLSearchParams {
  const params = new URLSearchParams();
  (Object.entries(filters) as [keyof LibraryFilters, string | undefined][]).forEach(([k, v]) => {
    if (v) params.set(k, v);
  });
  return params;
}
