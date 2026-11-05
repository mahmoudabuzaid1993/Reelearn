import { NextResponse, type NextRequest } from "next/server";
import { REFRESH_COOKIE } from "@/lib/auth/config";
import { rotateRefreshToken } from "@/lib/auth/server";
import { cookieOptions } from "../route";

/** Rotate the refresh cookie and hand a short-lived access token to the browser. */
export async function POST(request: NextRequest) {
  const current = request.cookies.get(REFRESH_COOKIE)?.value;
  if (!current) {
    return NextResponse.json({ title: "No session", status: 401 }, { status: 401 });
  }
  const pair = await rotateRefreshToken(current);
  if (!pair) {
    const res = NextResponse.json({ title: "Session expired", status: 401 }, { status: 401 });
    res.cookies.set(REFRESH_COOKIE, "", { ...cookieOptions(), maxAge: 0 });
    return res;
  }
  const res = NextResponse.json({
    access_token: pair.access_token,
    expires_in: pair.expires_in,
    user: pair.user,
  });
  res.cookies.set(REFRESH_COOKIE, pair.refresh_token, cookieOptions());
  return res;
}
