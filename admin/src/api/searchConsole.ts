import { createApi } from "./client";
import { isProblem } from "./posts";
import type { Reply } from "./redirects";
import type { components } from "./schema";

/** The long searches that showed each post in the last 28 days, and the heading that answers each. */
export type FollowUps = components["schemas"]["FollowUps"];
/** One post and its long searches. */
export type FollowUpPost = components["schemas"]["FollowUpPost"];
/** One long search and how it did. */
export type FollowUp = components["schemas"]["FollowUp"];

const api = createApi();

/**
 * Reads the follow-ups. Khalid asks Google on each call, so it can take some seconds.
 * A blog that cannot be reached answers status 0.
 */
export async function readFollowUps(): Promise<Reply<FollowUps>> {
  try {
    const { data, error, response } = await api.GET("/api/admin/v1/search-console/follow-ups/");
    if (data !== undefined) {
      return { data };
    }
    return { problem: isProblem(error) ? error : null, status: response.status };
  } catch {
    return { problem: null, status: 0 };
  }
}

/** How many of the searches no heading answers yet, and how many there are in all. */
export function openSearches({ posts }: FollowUps): { open: number; all: number } {
  const searches = posts.flatMap((post) => post.searches);
  const open = searches.filter((search) => search.answered_by == null).length;
  return { open, all: searches.length };
}
