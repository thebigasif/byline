// The super-admin routes of the Admin API (KH-EDT-7, plan 4.4), and the
// email sign-in they need. These routes answer a person only: a session
// cookie with its CSRF token, never a key. So the calls here send the
// token and leave the tab's key out, even when it has one.
import { readCsrf, saveCsrf } from "../auth/session";
import { isProblem } from "./posts";
import type { Reply } from "./redirects";
import type { components } from "./schema";

/** One site this server runs, as the Admin API sends it. */
export type Site = components["schemas"]["SiteAnswer"];
/** Who a sign-in signed in, and the session's CSRF token. */
export type Session = components["schemas"]["SessionAnswer"];

const ADMIN = "/api/admin/v1";

/** Sends one request with the session's CSRF token, and reads what came back. */
async function call<T>(
  path: string,
  init: RequestInit,
  read: (response: Response) => Promise<T>,
): Promise<Reply<T>> {
  const request = new Request(`${globalThis.location.origin}${ADMIN}${path}`, init);
  const csrf = readCsrf();
  if (csrf !== null) {
    request.headers.set("x-khalid-csrf", csrf);
  }
  try {
    const response = await globalThis.fetch(request);
    if (response.ok) {
      return { data: await read(response) };
    }
    const body: unknown = await response.json().catch(() => null);
    return { problem: isProblem(body) ? body : null, status: response.status };
  } catch {
    return { problem: null, status: 0 };
  }
}

/** A POST with `body` as JSON. */
function posted(body: unknown): RequestInit {
  return {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  };
}

/** Every site this server runs, the home site first. */
export function listSites(): Promise<Reply<Site[]>> {
  return call("/sites/", {}, async (response) => {
    const body: { sites: Site[] } = await response.json();
    return body.sites;
  });
}

/** Adds a site on `domain`; the server starts it at once. */
export function addSite(domain: string): Promise<Reply<Site>> {
  return call("/sites/", posted({ domain }), (response) => response.json());
}

/** Asks the blog to email a sign-in link; it answers the same for an unknown email. */
export function sendLink(email: string): Promise<Reply<null>> {
  return call("/session/link/", posted({ email }), async () => null);
}

/** Signs in with the token of a sign-in link, and keeps its CSRF token for this tab. */
export async function redeemLink(token: string): Promise<Reply<Session>> {
  const read = (response: Response): Promise<Session> => response.json();
  const reply = await call("/session/link/redeem/", posted({ token }), read);
  if ("data" in reply) {
    saveCsrf(reply.data.csrf);
  }
  return reply;
}
