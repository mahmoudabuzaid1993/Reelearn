import { tokenStore, type Me } from "./token-store";

export interface RefreshResponse {
  access_token: string;
  expires_in: number;
  user: Me;
}

let inflight: Promise<string | null> | null = null;

/** Rotate the httpOnly refresh cookie via the Next route handler and store the new access token in memory. */
export async function refreshAccessToken(): Promise<string | null> {
  if (inflight) return inflight;
  inflight = (async () => {
    try {
      const res = await fetch("/api/auth/session/refresh", { method: "POST", credentials: "same-origin" });
      if (!res.ok) {
        tokenStore.clear();
        return null;
      }
      const body = (await res.json()) as RefreshResponse;
      tokenStore.set(body);
      return body.access_token;
    } catch {
      tokenStore.clear();
      return null;
    } finally {
      inflight = null;
    }
  })();
  return inflight;
}

/** Persist the refresh token in the httpOnly cookie after an OTP verify. */
export async function persistSession(refreshToken: string): Promise<void> {
  await fetch("/api/auth/session", {
    method: "POST",
    credentials: "same-origin",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ refresh_token: refreshToken }),
  });
}

export async function endSession(): Promise<void> {
  tokenStore.clear();
  await fetch("/api/auth/session", { method: "DELETE", credentials: "same-origin" });
}
