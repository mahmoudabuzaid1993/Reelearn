"use client";

import { useMutation } from "@tanstack/react-query";
import { api } from "../client";
import { unwrap } from "../errors";
import { tokenStore } from "@/lib/auth/token-store";
import { persistSession } from "@/lib/auth/refresh";

export function useRequestOtp() {
  return useMutation({
    mutationFn: async (body: { tenant_slug: string; email?: string; phone?: string }) =>
      unwrap(await api.POST("/auth/otp/request", { body })),
  });
}

function deviceId(): string {
  if (typeof window === "undefined") return "server";
  const key = "reelearn.device";
  let id = window.localStorage.getItem(key);
  if (!id) {
    id = crypto.randomUUID();
    window.localStorage.setItem(key, id);
  }
  return id;
}

export function useVerifyOtp() {
  return useMutation({
    mutationFn: async (body: { challenge_id: string; code: string }) => {
      const pair = unwrap(
        await api.POST("/auth/otp/verify", {
          body: { ...body, device: { device_id: deviceId(), platform: "web", app_version: "web-0.1" } },
        }),
      );
      tokenStore.set(pair);
      await persistSession(pair.refresh_token);
      return pair;
    },
  });
}
