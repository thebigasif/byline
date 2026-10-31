import type { PostRow } from "../src/api/rows";
import { json } from "./render";

/** A row of the posts table, live and passing its checks unless `fields` say otherwise. */
export function row(fields: Partial<PostRow> = {}): PostRow {
  return {
    id: "01JA",
    slug: "why-internal-links-matter",
    title: "Why internal links matter",
    author: "asif",
    status: "published",
    review_state: "ready",
    reviewed: null,
    review_due: false,
    featured: false,
    published_at: "2026-09-14T09:00:00Z",
    updated_at: "2026-10-12T11:00:00Z",
    updated_by: "Asif",
    gate: "passes",
    lint: { blocks: 0, warnings: 1 },
    index_status: "indexable",
    ...fields,
  };
}

/** Answers a list request with `rows` as page `page` of a list `total` long. */
export function rowsPage(rows: PostRow[], total = rows.length, page = 1, limit = 15): Response {
  const pages = Math.max(1, Math.ceil(total / limit));
  const pagination = {
    page,
    limit,
    pages,
    total,
    next: page < pages ? page + 1 : null,
    prev: page > 1 ? page - 1 : null,
  };
  return json(200, { posts: rows, meta: { pagination } });
}

/** The query of a request that lists posts, or `null` for any other request. */
export function listing(request: Request): URLSearchParams | null {
  const url = new URL(request.url);
  const lists = request.method === "GET" && url.pathname === "/api/admin/v1/posts/";
  return lists ? url.searchParams : null;
}
