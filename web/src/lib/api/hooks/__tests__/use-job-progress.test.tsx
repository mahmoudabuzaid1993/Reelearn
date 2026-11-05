import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { GenerationJob } from "@/lib/api/types";
import { drainSseBuffer, useJobProgress } from "../use-job-progress";

const job = (status: GenerationJob["status"], pct: number): GenerationJob => ({
  id: "j1",
  source_id: "s1",
  status,
  progress_pct: pct,
  cost_credits: 10,
  steps: [],
  created_at: "2026-10-30T00:00:00Z",
});

function sseResponse(frames: GenerationJob[]): Response {
  const encoder = new TextEncoder();
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      frames.forEach((f) => controller.enqueue(encoder.encode(`event: snapshot\ndata: ${JSON.stringify(f)}\n\n`)));
      controller.close();
    },
  });
  return new Response(stream, { status: 200, headers: { "content-type": "text/event-stream" } });
}

describe("drainSseBuffer", () => {
  it("emits complete frames and keeps the partial tail", () => {
    const seen: string[] = [];
    const rest = drainSseBuffer('data: {"a":1}\n\nevent: x\ndata: {"b":2}\n\ndata: {"c":', (d) => seen.push(d));
    expect(seen).toEqual(['{"a":1}', '{"b":2}']);
    expect(rest).toBe('data: {"c":');
  });
});

describe("useJobProgress", () => {
  it("streams snapshots over SSE until a terminal status", async () => {
    const fetchFn = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/events")) return sseResponse([job("running", 40), job("done", 100)]);
      throw new Error("unexpected " + url);
    });
    const { result } = renderHook(() => useJobProgress("j1", { baseUrl: "http://api", headers: () => ({}), fetchFn: fetchFn as unknown as typeof fetch }));
    await waitFor(() => expect(result.current.job?.status).toBe("done"));
    expect(result.current.mode).toBe("sse");
    expect(fetchFn).toHaveBeenCalledTimes(1);
  });

  it("falls back to polling when the stream is unavailable", async () => {
    vi.useFakeTimers();
    let polls = 0;
    const fetchFn = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/events")) return new Response(null, { status: 404 });
      polls += 1;
      return Response.json(polls < 2 ? job("running", 50) : job("done", 100));
    });
    const { result } = renderHook(() => useJobProgress("j1", { baseUrl: "http://api", headers: () => ({}), pollMs: 50, fetchFn: fetchFn as unknown as typeof fetch }));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10);
    });
    expect(result.current.mode).toBe("polling");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(result.current.job?.status).toBe("done");
    expect(polls).toBe(2);
    vi.useRealTimers();
  });

  it("aborts in-flight work on unmount", async () => {
    const fetchFn = vi.fn((_input: RequestInfo | URL, init?: RequestInit) => new Promise<Response>((_resolve, reject) => init?.signal?.addEventListener("abort", () => reject(new DOMException("aborted", "AbortError")))));
    const { unmount, result } = renderHook(() => useJobProgress("j1", { baseUrl: "http://api", headers: () => ({}), fetchFn: fetchFn as unknown as typeof fetch }));
    unmount();
    expect(result.current.error).toBeNull();
  });
});
