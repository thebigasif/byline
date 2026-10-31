import { createApi } from "./client";
import { isProblem, type Problem } from "./posts";
import type { components } from "./schema";

/** A post's title and text as they were saved, before a later edit replaced them. */
export type PostVersion = components["schemas"]["PostVersion"];

const api = createApi();

/** What the versions call gave back: the versions, newest first, or the problem. */
export type Versions = { versions: PostVersion[] } | { problem: Problem | null; status: number };

/** The earlier versions of post `id`, the newest first. */
export async function listVersions(id: string): Promise<Versions> {
  const { data, error, response } = await api.GET("/api/admin/v1/posts/{id}/versions/", {
    params: { path: { id } },
  });
  if (data !== undefined) {
    return { versions: data.versions };
  }
  return { problem: isProblem(error) ? error : null, status: response.status };
}
