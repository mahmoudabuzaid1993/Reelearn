import { describe, expect, it, vi } from "vitest";
import { createTokenStore } from "../token-store";
import type { Me } from "../token-store";

const user = { id: "u1", role: "admin", full_name: "Noura", tenant: { id: "t", slug: "alwadi", name: "Al Wadi", languages: ["ar"] }, preferred_language: "ar", ui_language: "ar", delivery_channel: "app", delivery_slot: "shift_start" } as Me;

describe("token store", () => {
  it("keeps the access token in memory only and reports expiry with skew", () => {
    let now = 1_000_000;
    const store = createTokenStore(() => now);
    expect(store.getAccessToken()).toBeNull();
    expect(store.isExpired()).toBe(true);
    store.set({ access_token: "abc", expires_in: 60, user });
    expect(store.getAccessToken()).toBe("abc");
    expect(store.isExpired()).toBe(false);
    now += 50_000;
    expect(store.isExpired(15_000)).toBe(true);
    expect(store.isExpired(0)).toBe(false);
    expect(typeof window === "undefined" || window.localStorage.getItem("abc")).toBeFalsy();
  });

  it("notifies subscribers on set and clear", () => {
    const store = createTokenStore();
    const listener = vi.fn();
    const off = store.subscribe(listener);
    store.set({ access_token: "x", expires_in: 10, user });
    store.clear();
    expect(listener).toHaveBeenCalledTimes(2);
    expect(listener).toHaveBeenLastCalledWith(null);
    off();
    store.set({ access_token: "y", expires_in: 10, user });
    expect(listener).toHaveBeenCalledTimes(2);
  });
});
