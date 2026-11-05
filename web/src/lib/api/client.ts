import createClient, { type Middleware } from "openapi-fetch";
import type { paths } from "./schema";
import { apiBaseUrl } from "@/lib/auth/config";
import { tokenStore } from "@/lib/auth/token-store";
import { refreshAccessToken } from "@/lib/auth/refresh";

export const API_BASE = apiBaseUrl();

/** Routes that never carry a bearer token (security: [] in the contract). */
const ANONYMOUS = new Set(["/auth/tenant/lookup", "/auth/otp/request", "/auth/otp/verify", "/auth/refresh", "/tenants", "/health"]);

const authMiddleware: Middleware = {
  async onRequest({ request, schemaPath }) {
    if (ANONYMOUS.has(schemaPath)) return request;
    if (tokenStore.isExpired() && typeof window !== "undefined") {
      await refreshAccessToken();
    }
    const token = tokenStore.getAccessToken();
    if (token) request.headers.set("Authorization", `Bearer ${token}`);
    return request;
  },
  async onResponse({ response, schemaPath }) {
    if (response.status === 401 && !ANONYMOUS.has(schemaPath)) {
      tokenStore.clear();
    }
    return response;
  },
};

// Resolve fetch at call time so the mock layer (service worker or fetch fallback) is always honoured.
export const api = createClient<paths>({ baseUrl: API_BASE, fetch: (request) => globalThis.fetch(request) });
api.use(authMiddleware);

/** Headers for raw fetches that bypass the typed client (SSE, CSV upload). */
export function authHeaders(): HeadersInit {
  const token = tokenStore.getAccessToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}
