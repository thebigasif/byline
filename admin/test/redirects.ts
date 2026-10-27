import type { Miss, StoredRedirect } from "../src/api/redirects";
import { json, serve } from "./render";
import { rowsPage } from "./rows";

/** What the blog answers, keyed by method and route, such as `GET /redirects/`. */
export type Answers = Record<string, (request: Request) => Response | Promise<Response>>;

// What the Redirects screen reads when it opens, empty unless a test says otherwise.
const EMPTY: Answers = {
  "GET /not-found/": () => json(200, { misses: [] }),
  "GET /redirects/": () => json(200, { redirects: [] }),
  "GET /posts/": () => rowsPage([]),
};

/** A blog that answers Admin API routes from `answers`, and the requests it was sent. */
export function redirectsBlog(answers: Answers) {
  const sent: Request[] = [];
  const all = { ...EMPTY, ...answers };
  serve((request) => {
    sent.push(request);
    const route = new URL(request.url).pathname.replace("/api/admin/v1", "");
    return all[`${request.method} ${route}`]?.(request) ?? json(404, {});
  });
  return sent;
}

/** A redirect Khalid keeps: a permanent move made by hand unless `fields` say otherwise. */
export function kept(fields: Partial<StoredRedirect> = {}): StoredRedirect {
  return {
    id: 12,
    from: "/blog/internal-linking/",
    to: "/blog/why-internal-links-matter/",
    status: 301,
    match: "exact",
    origin: "manual",
    created_at: "2026-10-20T09:00:00Z",
    ...fields,
  };
}

/** An address that answered 404, with a page near it unless `fields` say otherwise. */
export function miss(fields: Partial<Miss> = {}): Miss {
  return {
    path: "/blog/internal-links/",
    hits: 42,
    first_seen: "2026-10-01T08:00:00Z",
    last_seen: "2026-10-27T07:30:00Z",
    nearest: "/blog/why-internal-links-matter/",
    ...fields,
  };
}
