import { getResponse } from "msw";
import { handlers } from "./handlers";
import { apiBaseUrl } from "@/lib/auth/config";

/**
 * Some embedded browsers refuse service-worker registration. In that case we resolve API
 * requests against the same MSW handlers from inside `window.fetch`, so mock mode still works.
 */
export function installFetchFallback(): void {
  if (typeof window === "undefined") return;
  const original = window.fetch.bind(window);
  const base = apiBaseUrl();
  window.fetch = async (input, init) => {
    const request = new Request(input, init);
    if (!request.url.startsWith(base)) return original(input, init);
    const response = await getResponse(handlers, request);
    return response ?? original(input, init);
  };
}
