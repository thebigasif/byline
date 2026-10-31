// The trash and the audit log of the Admin API, and the calls the posts
// table makes for many posts at once (KH-EDT-6).
import { createApi } from "./client";
import { type AdminPost, isProblem } from "./posts";
import type { Failure, Reply } from "./redirects";
import type { components } from "./schema";

/** A deleted post waiting in the trash. */
export type TrashedPost = components["schemas"]["TrashedPost"];
/** One change in the audit log. */
export type Change = components["schemas"]["AdminChange"];
/** One page of the audit log, and the `before` that lists the next. */
export type Changes = components["schemas"]["AdminChanges"];

const api = createApi();

/** The reply to a call that failed: the problem the API sent, if any. */
function failed(error: unknown, response: Response): Failure {
  return { problem: isProblem(error) ? error : null, status: response.status };
}

/** The reply when the blog cannot be reached at all. */
const UNREACHABLE: Failure = { problem: null, status: 0 };

/** Lists the deleted posts, the most recently deleted first. */
export async function listTrash(): Promise<Reply<TrashedPost[]>> {
  try {
    const { data, error, response } = await api.GET("/api/admin/v1/trash/");
    return data === undefined ? failed(error, response) : { data: data.trash };
  } catch {
    return UNREACHABLE;
  }
}

/** Puts a deleted post back where it was, and answers it as it is now. */
export async function restoreTrashed(key: string): Promise<Reply<AdminPost>> {
  try {
    const { data, error, response } = await api.POST("/api/admin/v1/trash/{key}/restore/", {
      params: { path: { key } },
    });
    const post = data?.posts[0];
    return post === undefined ? failed(error, response) : { data: post };
  } catch {
    return UNREACHABLE;
  }
}

/** Deletes a post in the trash for good. */
export async function purgeTrashed(key: string): Promise<Reply<null>> {
  try {
    const { error, response } = await api.DELETE("/api/admin/v1/trash/{key}/", {
      params: { path: { key } },
    });
    return response.ok ? { data: null } : failed(error, response);
  } catch {
    return UNREACHABLE;
  }
}

/** Moves a post to the trash, if it has not changed since the copy `updatedAt` names. */
export async function trashPost(id: string, updatedAt: string): Promise<Reply<null>> {
  try {
    const { error, response } = await api.DELETE("/api/admin/v1/posts/{id}/", {
      params: { path: { id }, query: { updated_at: updatedAt } },
    });
    return response.ok ? { data: null } : failed(error, response);
  } catch {
    return UNREACHABLE;
  }
}

/** One page of the audit log, newest first, older than the change `before` when given. */
export async function listChanges(before?: number): Promise<Reply<Changes>> {
  try {
    const { data, error, response } = await api.GET("/api/admin/v1/audit/", {
      params: { query: { before } },
    });
    return data === undefined ? failed(error, response) : { data };
  } catch {
    return UNREACHABLE;
  }
}
