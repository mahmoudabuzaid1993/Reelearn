import "server-only";
import { apiBaseUrl, isMockMode } from "./config";
import type { components } from "@/lib/api/schema";

type TokenPair = components["schemas"]["TokenPair"];

/**
 * Exchange a refresh token for a new pair. Runs on the server only, so the
 * refresh token never reaches browser JavaScript.
 */
export async function rotateRefreshToken(refreshToken: string): Promise<TokenPair | null> {
  if (isMockMode()) {
    const { mockRotateRefreshToken } = await import("@/lib/mocks/auth");
    return mockRotateRefreshToken(refreshToken);
  }
  const res = await fetch(`${apiBaseUrl()}/auth/refresh`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ refresh_token: refreshToken }),
    cache: "no-store",
  });
  if (!res.ok) return null;
  return (await res.json()) as TokenPair;
}

export async function revokeSession(accessToken: string | null): Promise<void> {
  if (isMockMode() || !accessToken) return;
  await fetch(`${apiBaseUrl()}/auth/logout`, {
    method: "POST",
    headers: { authorization: `Bearer ${accessToken}`, "content-type": "application/json" },
    body: JSON.stringify({ all_devices: false }),
  }).catch(() => undefined);
}
