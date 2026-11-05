import type { TokenPair, User } from "@/lib/api/types";
import { meFor, users } from "./fixtures/tenant";

export const DEV_OTP_CODE = "1234";
export const ACCESS_PREFIX = "mock-access-";
export const REFRESH_PREFIX = "mock-refresh-";

export function userFromToken(token: string | null | undefined, prefix = ACCESS_PREFIX): User | null {
  if (!token || !token.startsWith(prefix)) return null;
  const id = token.slice(prefix.length);
  return users.find((u) => u.id === id) ?? null;
}

export function tokenPairFor(user: User): TokenPair {
  return {
    access_token: `${ACCESS_PREFIX}${user.id}`,
    refresh_token: `${REFRESH_PREFIX}${user.id}`,
    expires_in: 900,
    user: meFor(user),
  };
}

/** Used by the Next route handler in mock mode (server side, no MSW). */
export function mockRotateRefreshToken(refreshToken: string): TokenPair | null {
  const user = userFromToken(refreshToken, REFRESH_PREFIX);
  return user ? tokenPairFor(user) : null;
}
