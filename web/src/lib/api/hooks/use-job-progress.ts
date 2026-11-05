"use client";

import { useEffect, useRef, useState } from "react";
import type { GenerationJob } from "../types";
import { isTerminal } from "@/features/studio/logic/job-status";

export type ProgressMode = "sse" | "polling" | "idle";

export interface JobProgressOptions {
  /** Absolute base URL of the API. */
  baseUrl: string;
  headers: () => HeadersInit;
  /** Polling interval used after SSE fails. */
  pollMs?: number;
  /** Injectable for tests. */
  fetchFn?: typeof fetch;
  initial?: GenerationJob | null;
  /** Bump to re-subscribe (for example after a retry). */
  epoch?: number;
}

/** Parse one `data:` payload out of an SSE chunk buffer; returns remaining buffer. */
export function drainSseBuffer(buffer: string, onData: (data: string) => void): string {
  let rest = buffer;
  let idx = rest.indexOf("\n\n");
  while (idx >= 0) {
    const frame = rest.slice(0, idx);
    rest = rest.slice(idx + 2);
    const data = frame
      .split("\n")
      .filter((line) => line.startsWith("data:"))
      .map((line) => line.slice(5).trim())
      .join("\n");
    if (data) onData(data);
    idx = rest.indexOf("\n\n");
  }
  return rest;
}

/**
 * Live job progress: streams `/events` over fetch (works through MSW and proxies),
 * and falls back to polling `GET /generation-jobs/{id}` if the stream fails or is unsupported.
 */
export function useJobProgress(jobId: string, options: JobProgressOptions) {
  const { baseUrl, headers, pollMs = 2000, fetchFn, initial = null, epoch = 0 } = options;
  const [job, setJob] = useState<GenerationJob | null>(initial);
  const [mode, setMode] = useState<ProgressMode>("idle");
  const [error, setError] = useState<string | null>(null);
  const headersRef = useRef(headers);
  headersRef.current = headers;

  useEffect(() => {
    if (!jobId) return;
    const doFetch = fetchFn ?? fetch;
    const controller = new AbortController();
    let pollTimer: ReturnType<typeof setTimeout> | null = null;
    let stopped = false;

    const stop = () => {
      stopped = true;
      controller.abort();
      if (pollTimer) clearTimeout(pollTimer);
    };

    const poll = async () => {
      if (stopped) return;
      setMode("polling");
      try {
        const res = await doFetch(`${baseUrl}/admin/generation-jobs/${jobId}`, { headers: headersRef.current(), signal: controller.signal });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const next = (await res.json()) as GenerationJob;
        setJob(next);
        setError(null);
        if (isTerminal(next.status)) return;
      } catch (err) {
        if (controller.signal.aborted) return;
        setError(err instanceof Error ? err.message : "Polling failed");
      }
      pollTimer = setTimeout(poll, pollMs);
    };

    const stream = async () => {
      try {
        const res = await doFetch(`${baseUrl}/admin/generation-jobs/${jobId}/events`, {
          headers: { ...headersRef.current(), accept: "text/event-stream" },
          signal: controller.signal,
        });
        if (!res.ok || !res.body) throw new Error(`SSE unavailable (${res.status})`);
        setMode("sse");
        const reader = res.body.getReader();
        const decoder = new TextDecoder();
        let buffer = "";
        let terminal = false;
        for (;;) {
          const { value, done } = await reader.read();
          if (done) break;
          buffer += decoder.decode(value, { stream: true });
          buffer = drainSseBuffer(buffer, (data) => {
            try {
              const next = JSON.parse(data) as GenerationJob;
              setJob(next);
              setError(null);
              if (isTerminal(next.status)) terminal = true;
            } catch {
              // ignore malformed frames
            }
          });
          if (terminal) break;
        }
        if (!terminal && !stopped) void poll();
      } catch (err) {
        if (controller.signal.aborted) return;
        setError(err instanceof Error ? err.message : "Stream failed");
        void poll();
      }
    };

    void stream();
    return stop;
  }, [jobId, baseUrl, pollMs, fetchFn, epoch]);

  return { job, mode, error };
}
