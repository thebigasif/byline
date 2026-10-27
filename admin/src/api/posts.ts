import type { UserMessage } from "../strings/en";
import { createApi } from "./client";
import type { components } from "./schema";

/** A post as the Admin API sends it, in any status. */
export type AdminPost = components["schemas"]["AdminPost"];
/** The fields an edit may change, and the copy it was made on. */
export type PostEdit = components["schemas"]["PostEdit"];
/** An error as the API sends it, with what, why and fix. */
export type Problem = components["schemas"]["Problem"];

const api = createApi();

/** What an Admin API call gave back: the post, or the problem. */
export type Answer = { post: AdminPost } | { problem: Problem | null; status: number };

/** Whether a body is a problem the API sent, as RULES.md BL-ERR-5 shapes it. */
export function isProblem(body: unknown): body is Problem {
  return (
    typeof body === "object" &&
    body !== null &&
    "detail" in body &&
    typeof body.detail === "string" &&
    "fix" in body &&
    typeof body.fix === "string"
  );
}

/** What to tell the writer about a failed call: the API's words, or `fallback`. */
export function problemMessage(problem: Problem | null, fallback: UserMessage): UserMessage {
  if (problem === null) {
    return fallback;
  }
  return { what: problem.title, why: problem.detail, fix: problem.fix };
}

/** A fresh `Idempotency-Key`; works on plain http too, unlike randomUUID. */
export function requestKey(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function answer(
  data: { posts: AdminPost[] } | undefined,
  error: unknown,
  response: Response,
): Answer {
  const post = data?.posts[0];
  if (post !== undefined) {
    return { post };
  }
  return { problem: isProblem(error) ? error : null, status: response.status };
}

/** Reads one post by its identity. */
export async function readPost(id: string): Promise<Answer> {
  const { data, error, response } = await api.GET("/api/admin/v1/posts/{id}/", {
    params: { path: { id } },
  });
  return answer(data, error, response);
}

/** Creates a draft with only a title; `key` makes a retry harmless. */
export async function createPost(title: string, key: string): Promise<Answer> {
  const { data, error, response } = await api.POST("/api/admin/v1/posts/", {
    params: { header: { "Idempotency-Key": key } },
    body: { posts: [{ title, markdown: "" }] },
  });
  return answer(data, error, response);
}

/** A link that shows a draft to anyone who has it, and when it stops working. */
export type PreviewLink = components["schemas"]["PreviewLink"];

/** Makes a preview link for one post. */
export async function makePreviewLink(
  id: string,
): Promise<{ link: PreviewLink } | { problem: Problem | null; status: number }> {
  const { data, error, response } = await api.POST("/api/admin/v1/posts/{id}/preview-link/", {
    params: { path: { id } },
  });
  const link = data?.preview_links[0];
  if (link !== undefined) {
    return { link };
  }
  return { problem: isProblem(error) ? error : null, status: response.status };
}

/** Saves an edit made on the copy that `edit.updated_at` names. */
export async function savePost(id: string, edit: PostEdit): Promise<Answer> {
  const { data, error, response } = await api.PUT("/api/admin/v1/posts/{id}/", {
    params: { path: { id } },
    body: { posts: [edit] },
  });
  return answer(data, error, response);
}
