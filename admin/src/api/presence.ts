import { createApi } from "./client";
import type { components } from "./schema";

/** Who has a post open, and who holds its lock, as this writer sees it. */
export type PostPresence = components["schemas"]["PostPresence"];

/** What a check-in asks for: the lock if it is free, or the lock from whoever holds it. */
export type Claim = components["schemas"]["Claim"];

const api = createApi();

/** Whether an answer is a presence; an older blog answers something else or nothing. */
function isPresence(data: unknown): data is PostPresence {
  return typeof data === "object" && data !== null && "holder" in data && "yours" in data;
}

/** Notes that this writer has post `id` open; `null` when the blog cannot say who holds it. */
export async function checkIn(id: string, claim: Claim): Promise<PostPresence | null> {
  const { data } = await api.POST("/api/admin/v1/posts/{id}/presence/", {
    params: { path: { id } },
    body: { claim },
  });
  return isPresence(data) ? data : null;
}

/**
 * Notes that this writer closed post `id`; sent even while the tab closes.
 * A leave that does not arrive is harmless: the lock lets go a minute later.
 */
export async function leave(id: string): Promise<void> {
  try {
    await api.DELETE("/api/admin/v1/posts/{id}/presence/", {
      params: { path: { id } },
      keepalive: true,
    });
  } catch {
    // Nothing to tell the writer, who has left already.
  }
}
