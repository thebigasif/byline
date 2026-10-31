import { problemMessage, savePost } from "../api/posts";
import { trashPost } from "../api/records";
import { isData } from "../api/redirects";
import type { PostRow } from "../api/rows";
import { en } from "../strings/en";

/** What one change to many posts does to each of them. */
export type BulkAction = { kind: "author"; author: string } | { kind: "trash" };

/** What a change to many posts came to: how many changed, and why each other one did not. */
export type BulkResult = { changed: number; left: string[] };

/**
 * Changes each of `rows` on its own, each from the copy its row was read from, so a post
 * someone else changed meanwhile is left alone and one refusal never stops the rest.
 */
export async function changeAll(rows: PostRow[], action: BulkAction): Promise<BulkResult> {
  const whys = await Promise.all(rows.map((row) => changeOne(row, action)));
  const left = rows.flatMap((row, at) => {
    const why = whys[at];
    return why === null || why === undefined ? [] : [en.bulk.left(row.title, why)];
  });
  return { changed: rows.length - left.length, left };
}

/** Changes one post: `null` when it changed, or else why not, in words. */
async function changeOne(row: PostRow, action: BulkAction): Promise<string | null> {
  if (action.kind === "trash") {
    const reply = await trashPost(row.id, row.updated_at);
    return isData(reply) ? null : problemMessage(reply.problem, en.problems.unreachable).what;
  }
  const edit = { updated_at: row.updated_at, author: action.author };
  const answer = await savePost(row.id, edit);
  return "post" in answer ? null : problemMessage(answer.problem, en.problems.unreachable).what;
}
