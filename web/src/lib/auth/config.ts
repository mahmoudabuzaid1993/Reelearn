export const REFRESH_COOKIE = process.env.REELEARN_REFRESH_COOKIE ?? "rl_refresh";
export const PUBLIC_PATHS = ["/signin", "/signup"] as const;
export const DEFAULT_AFTER_SIGNIN = "/dashboard";

export function isMockMode(): boolean {
  return process.env.NEXT_PUBLIC_API_MOCK === "true";
}

export function apiBaseUrl(): string {
  return process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080/v1";
}
