"use client";

import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { useRouter } from "next/navigation";
import { tokenStore, type Me, type Session } from "./token-store";
import { endSession, refreshAccessToken } from "./refresh";

export type AuthStatus = "loading" | "authenticated" | "anonymous";

interface AuthContextValue {
  status: AuthStatus;
  user: Me | null;
  signOut: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const router = useRouter();
  const [session, setSession] = useState<Session | null>(() => tokenStore.get());
  const [status, setStatus] = useState<AuthStatus>(() => (tokenStore.get() ? "authenticated" : "loading"));

  useEffect(() => {
    const unsubscribe = tokenStore.subscribe((next) => {
      setSession(next);
      setStatus(next ? "authenticated" : "anonymous");
    });
    if (!tokenStore.get()) {
      void refreshAccessToken().then((token) => {
        if (!token) setStatus("anonymous");
      });
    }
    return unsubscribe;
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({
      status,
      user: session?.user ?? null,
      signOut: async () => {
        await endSession();
        router.replace("/signin");
      },
    }),
    [status, session, router],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used inside AuthProvider");
  return ctx;
}
