import { NextResponse, type NextRequest } from "next/server";
import { DEFAULT_AFTER_SIGNIN, PUBLIC_PATHS, REFRESH_COOKIE } from "@/lib/auth/config";

/** Route guard: admins without a refresh cookie are sent to /signin. */
export function middleware(request: NextRequest) {
  const { pathname } = request.nextUrl;
  const hasSession = Boolean(request.cookies.get(REFRESH_COOKIE)?.value);
  const isPublic = PUBLIC_PATHS.some((p) => pathname === p || pathname.startsWith(`${p}/`));

  if (!hasSession && !isPublic) {
    const url = request.nextUrl.clone();
    url.pathname = "/signin";
    url.search = pathname === "/" ? "" : `?next=${encodeURIComponent(pathname + request.nextUrl.search)}`;
    return NextResponse.redirect(url);
  }
  if (hasSession && pathname === "/signin") {
    const url = request.nextUrl.clone();
    url.pathname = DEFAULT_AFTER_SIGNIN;
    url.search = "";
    return NextResponse.redirect(url);
  }
  return NextResponse.next();
}

export const config = {
  matcher: ["/((?!api|_next/static|_next/image|favicon.ico|mockServiceWorker.js|.*\\..*).*)"],
};
