import type { PostRow } from "../api/rows";

/** Where a post is on its way to readers. */
export const STAGES = ["draft", "in_review", "ready", "scheduled", "live"] as const;

/** A post's stage. */
export type Stage = (typeof STAGES)[number];

/** A row's stage: live or scheduled by its status, else its review state. */
export function stageOf(row: PostRow): Stage {
  if (row.status === "published") {
    return "live";
  }
  return row.status === "scheduled" ? "scheduled" : row.review_state;
}
