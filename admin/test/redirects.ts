import type { Miss, StoredRedirect } from "../src/api/redirects";
import { type Answers, adminBlog } from "./blog";
import { json } from "./render";
import { rowsPage } from "./rows";

export type { Answers } from "./blog";

// What the Redirects screen reads when it opens, empty unless a test says otherwise.
const EMPTY: Answers = {
  "GET /not-found/": () => json(200, { misses: [] }),
  "GET /redirects/": () => json(200, { redirects: [] }),
  "GET /posts/": () => rowsPage([]),
};

/** A blog that answers Admin API routes from `answers`, and the requests it was sent. */
export function redirectsBlog(answers: Answers) {
  return adminBlog({ ...EMPTY, ...answers });
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
