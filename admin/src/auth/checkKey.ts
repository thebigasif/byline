import { createApi } from "../api/client";

/** What the blog said about a key someone typed. */
export type KeyCheck = "accepted" | "rejected" | "lacks-scope" | "unreachable";

// No post has this identity, so a key the blog accepts gets a 404 back.
const PROBE_ID = "khalid-admin-sign-in-check";

/**
 * Asks the Admin API whether a key works, by reading a post that cannot
 * exist: a 404 means the key got past the key check and the scope check.
 */
export async function checkKey(
  key: string,
  fetch: typeof globalThis.fetch = globalThis.fetch,
): Promise<KeyCheck> {
  const api = createApi({ key: () => key, fetch });
  try {
    const { response } = await api.GET("/api/admin/v1/posts/{id}/", {
      params: { path: { id: PROBE_ID } },
    });
    switch (response.status) {
      case 200:
      case 404:
        return "accepted";
      case 401:
        return "rejected";
      case 403:
        return "lacks-scope";
      default:
        return "unreachable";
    }
  } catch {
    return "unreachable";
  }
}
