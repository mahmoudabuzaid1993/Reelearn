import type { Problem } from "./types";

/** An RFC 9457 problem returned by the API, with the request id for support. */
export class ApiError extends Error {
  readonly status: number;
  readonly requestId: string | null;
  readonly problem: Problem | null;
  readonly fieldErrors: Record<string, string>;

  constructor(status: number, problem: Problem | null, fallback = "Request failed") {
    super(problem?.detail ?? problem?.title ?? fallback);
    this.name = "ApiError";
    this.status = status;
    this.problem = problem;
    this.requestId = problem?.request_id ?? null;
    this.fieldErrors = Object.fromEntries(
      (problem?.errors ?? [])
        .filter((e): e is { field: string; message: string } => Boolean(e.field && e.message))
        .map((e) => [e.field, e.message]),
    );
  }
}

/** The contract's Page schema marks `items` optional in the generated type; normalise it. */
export function asPage<T>(data: { items?: T[]; next_cursor?: string | null }): { items: T[]; next_cursor: string | null } {
  return { items: data.items ?? [], next_cursor: data.next_cursor ?? null };
}

export function isApiError(error: unknown): error is ApiError {
  return error instanceof ApiError;
}

/**
 * openapi-fetch returns `{ data, error, response }`; this turns an error branch into a thrown ApiError
 * so TanStack Query sees a rejected promise.
 */
export function unwrap<T>(result: { data?: T; error?: unknown; response: Response }): T {
  if (result.error !== undefined || !result.response.ok) {
    const problem = (result.error ?? null) as Problem | null;
    throw new ApiError(result.response.status, problem && typeof problem === "object" ? problem : null);
  }
  if (result.data === undefined) {
    // 204 No Content and similar
    return undefined as T;
  }
  return result.data;
}
