import { createApi } from "./client";
import type { components } from "./schema";

/** A post checked as it would go live now: lint, score, search result and schema. */
export type PostCheck = components["schemas"]["PostCheck"];
/** The lines of a search result for a post. */
export type Snippet = components["schemas"]["Snippet"];

const api = createApi();

/**
 * Checks one post as it would go live now, saving nothing. Null when the blog cannot check
 * it, so the editor shows the checks it already has and nothing more.
 */
export async function readCheck(id: string): Promise<PostCheck | null> {
  const { data } = await api
    .GET("/api/admin/v1/posts/{id}/check/", { params: { path: { id } } })
    .catch(() => ({ data: undefined }));
  const checks: unknown = data?.checks;
  if (!Array.isArray(checks)) {
    return null;
  }
  const check: PostCheck | undefined = checks[0];
  return check ?? null;
}
