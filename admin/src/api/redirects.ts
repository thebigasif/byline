// The redirect routes of the Admin API (KH-SEO-8). They are typed by hand,
// field for field as `AdminRedirect` and `AdminMiss`, because the OpenAPI
// document gains them only when the redirects track lands. Then these
// types become `components["schemas"]` and the calls go through `createApi`.
import { readKey } from "../auth/session";
import { isProblem, type Problem } from "./posts";

/** A redirect Khalid keeps in its database, as the Admin API sends it. */
export type StoredRedirect = {
  id: number;
  /** The path it starts from, or a regular expression over the path. */
  from: string;
  /** Where a move leads; `null` for 410 and 451. */
  to: string | null;
  /** 301, 302, 307, 410 or 451. */
  status: number;
  match: "exact" | "regex";
  /** Who made it: a person, an import, or Khalid when a post's address changed. */
  origin: "manual" | "import" | "slug_change";
  created_at: string;
};

/** An address on the blog that answered 404, with the page nearest to it. */
export type Miss = {
  path: string;
  hits: number;
  first_seen: string;
  last_seen: string;
  /** The page "redirect this" sends it to, or `null` when none is near enough to guess. */
  nearest: string | null;
};

/** A call that failed: the problem the API sent, if any, and the status; 0 when unreachable. */
export type Failure = { problem: Problem | null; status: number };

/** What a redirect call gave back: its data, or the problem. */
export type Reply<T> = { data: T } | Failure;

/** Whether a call gave back its data. */
export function isData<T>(reply: Reply<T> | undefined): reply is { data: T } {
  return reply !== undefined && "data" in reply;
}

const ADMIN = "/api/admin/v1";

/** Sends one request to the blog with the signed-in key, and reads what came back. */
async function call<T>(
  path: string,
  init: RequestInit,
  read: (response: Response) => Promise<T>,
): Promise<Reply<T>> {
  const request = new Request(`${globalThis.location.origin}${ADMIN}${path}`, init);
  const key = readKey();
  if (key !== null) {
    request.headers.set("Authorization", `Khalid ${key}`);
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

async function redirects(response: Response): Promise<StoredRedirect[]> {
  const body: { redirects: StoredRedirect[] } = await response.json();
  return body.redirects;
}

/** Every redirect Khalid keeps, oldest first; aliases in post files are not among them. */
export function listRedirects(): Promise<Reply<StoredRedirect[]>> {
  return call("/redirects/", {}, redirects);
}

/** Removes one kept redirect. */
export function removeRedirect(id: number): Promise<Reply<null>> {
  return call(`/redirects/${id}/`, { method: "DELETE" }, async () => null);
}

/** Stores every redirect in a CSV list, or none; gives back those stored. */
export function importRedirects(csv: string): Promise<Reply<StoredRedirect[]>> {
  const init = { method: "POST", headers: { "Content-Type": "text/csv" }, body: csv };
  return call("/redirects/import/", init, redirects);
}

/** Every kept redirect as CSV text, header first. */
export function exportRedirects(): Promise<Reply<string>> {
  return call("/redirects/export/", {}, (response) => response.text());
}

/** The addresses that answered 404, most visited first. */
export function listMisses(): Promise<Reply<Miss[]>> {
  return call("/not-found/", {}, async (response) => {
    const body: { misses: Miss[] } = await response.json();
    return body.misses;
  });
}

/** Sends an address that answered 404 to `to` for good, and drops it from the log. */
export function redirectMiss(path: string, to: string): Promise<Reply<StoredRedirect[]>> {
  const init = {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path, to }),
  };
  return call("/not-found/redirect/", init, redirects);
}
