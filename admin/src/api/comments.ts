import { call, nothing, type Reply, sending } from "./call";
import type { components } from "./schema";

/** A comment as the Admin API sends it, in any status. */
export type AdminComment = components["schemas"]["AdminComment"];
/** Where a comment stands in moderation: pending, approved, spam or trash. */
export type CommentStatus = components["schemas"]["CommentStatus"];

async function comments(response: Response): Promise<AdminComment[]> {
  const body: components["schemas"]["AdminComments"] = await response.json();
  return body.comments;
}

/** The newest 100 comments with one status. */
export function listComments(status: CommentStatus): Promise<Reply<AdminComment[]>> {
  return call(`/comments/?status=${status}&limit=100`, {}, comments);
}

/** Shows a comment under its post. */
export function approveComment(id: number): Promise<Reply<AdminComment[]>> {
  return call(`/comments/${id}/approve/`, { method: "POST" }, comments);
}

/** Answers a comment as staff; the answer shows at once and approves what it answers. */
export function replyToComment(
  id: number,
  name: string,
  text: string,
): Promise<Reply<AdminComment[]>> {
  const body: components["schemas"]["CommentReplies"] = { comments: [{ author_name: name, text }] };
  return call(`/comments/${id}/replies/`, sending("POST", body), comments);
}

/** Removes a comment and every reply to it. */
export function deleteComment(id: number): Promise<Reply<null>> {
  return call(`/comments/${id}/`, { method: "DELETE" }, nothing);
}
