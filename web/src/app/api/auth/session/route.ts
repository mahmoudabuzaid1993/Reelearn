import { NextResponse, type NextRequest } from "next/server";
import { REFRESH_COOKIE } from "@/lib/auth/config";
import { revokeSession } from "@/lib/auth/server";

const COOKIE_MAX_AGE = 60 * 60 * 24 * 30;

export function cookieOptions() {
  return {
    httpOnly: true,
    sameSite: "lax" as const,
    secure: process.env.NODE_ENV === "production",
    path: "/",
    maxAge: COOKIE_MAX_AGE,
  };
}

/** Store the refresh token from an OTP verify in an httpOnly cookie. */
export async function POST(request: NextRequest) {
  const body = (await request.json().catch(() => null)) as { refresh_token?: string } | null;
  if (!body?.refresh_token) {
    return NextResponse.json({ title: "refresh_token required", status: 400 }, { status: 400 });
  }
  const res = new NextResponse(null, { status: 204 });
  res.cookies.set(REFRESH_COOKIE, body.refresh_token, cookieOptions());
  return res;
}

/** Sign out: revoke upstream (best effort) and drop the cookie. */
export async function DELETE(request: NextRequest) {
  const bearer = request.headers.get("authorization")?.replace(/^Bearer\s+/i, "") ?? null;
  await revokeSession(bearer);
  const res = new NextResponse(null, { status: 204 });
  res.cookies.set(REFRESH_COOKIE, "", { ...cookieOptions(), maxAge: 0 });
  return res;
}
