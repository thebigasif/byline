import type { RowQuery } from "../api/rows";
import { STAGES, type Stage } from "./stages";

/** The checks filter: posts the linter stops, or posts nothing stops. */
export const CHECKS = ["blocked", "clear"] as const;
/** What the site tells search engines about a post. */
export const INDEX = ["indexable", "noindex", "canonical_elsewhere", "not_live"] as const;

/** A checks filter. */
export type Checks = (typeof CHECKS)[number];
/** What search engines are told. */
export type Index = (typeof INDEX)[number];

/** The posts screen's address: its filters and page, each left out when not picked. */
export type PostSearch = { checks?: Checks; stage?: Stage; index?: Index; page?: number };

/** How many rows one page of the posts screen holds. */
export const PAGE_SIZE = 20;

const STAGE_FILTERS: Record<Stage, string> = {
  draft: "status:draft+review_state:draft",
  in_review: "status:draft+review_state:in_review",
  ready: "status:draft+review_state:ready",
  scheduled: "status:scheduled",
  live: "status:published",
};

function oneOf<T extends string>(known: readonly T[], value: unknown): T | undefined {
  return known.find((each) => each === value);
}

/** Reads the posts screen's address, dropping anything it does not know. */
export function postSearch(search: Record<string, unknown>): PostSearch {
  const page = Number(search.page);
  return {
    checks: oneOf(CHECKS, search.checks),
    stage: oneOf(STAGES, search.stage),
    index: oneOf(INDEX, search.index),
    page: Number.isInteger(page) && page > 1 ? page : undefined,
  };
}

/** The list query for the posts screen's address: newest change first. */
export function rowQuery(search: PostSearch): RowQuery {
  const parts: string[] = [];
  if (search.checks !== undefined) {
    parts.push(search.checks === "blocked" ? "gate:refuses" : "gate:passes");
  }
  if (search.stage !== undefined) {
    parts.push(STAGE_FILTERS[search.stage]);
  }
  if (search.index !== undefined) {
    parts.push(`index_status:${search.index}`);
  }
  const filter = parts.length === 0 ? undefined : parts.join("+");
  return { filter, order: "updated_at desc", limit: PAGE_SIZE, page: search.page ?? 1 };
}
