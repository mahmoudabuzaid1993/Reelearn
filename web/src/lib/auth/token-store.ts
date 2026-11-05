import type { components } from "@/lib/api/schema";

export type Me = components["schemas"]["Me"];

export interface Session {
  accessToken: string;
  /** Epoch ms when the access token expires. */
  expiresAt: number;
  user: Me;
}

type Listener = (session: Session | null) => void;

/**
 * Access tokens for admins live only in memory (never localStorage); the refresh
 * token is an httpOnly cookie handled by the /api/auth/session route handlers.
 */
export function createTokenStore(now: () => number = () => Date.now()) {
  let session: Session | null = null;
  const listeners = new Set<Listener>();

  const emit = () => listeners.forEach((l) => l(session));

  return {
    get(): Session | null {
      return session;
    },
    getAccessToken(): string | null {
      return session?.accessToken ?? null;
    },
    set(next: { access_token: string; expires_in: number; user: Me }): Session {
      session = {
        accessToken: next.access_token,
        expiresAt: now() + next.expires_in * 1000,
        user: next.user,
      };
      emit();
      return session;
    },
    clear(): void {
      session = null;
      emit();
    },
    /** True when the token is missing or expires within `skewMs`. */
    isExpired(skewMs = 15_000): boolean {
      return !session || session.expiresAt - skewMs <= now();
    },
    subscribe(listener: Listener): () => void {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

export const tokenStore = createTokenStore();
