import { useState } from "react";
import type { TrashedPost } from "../api/records";
import { moment } from "../strings/dates";
import { en } from "../strings/en";

type Props = {
  trash: TrashedPost[];
  onRestore: (post: TrashedPost) => void;
  onPurge: (post: TrashedPost) => void;
};

/** The deleted posts: title, what each was, when it went, and what to do with it. */
export function TrashTable({ trash, onRestore, onPurge }: Props) {
  const words = en.trash;
  // Deleting for good cannot be undone, so it asks once more, in the post's own row.
  const [asking, setAsking] = useState<string | null>(null);
  return (
    <div className="scroll">
      <table aria-labelledby="trash-title">
        <thead>
          <tr>
            <th scope="col">{words.titleColumn}</th>
            <th scope="col">{words.was}</th>
            <th scope="col">{words.deleted}</th>
            <th scope="col">{words.action}</th>
          </tr>
        </thead>
        <tbody>
          {trash.map((post) => (
            <tr key={post.key}>
              <td>{post.title}</td>
              <td data-label={words.was}>
                <span className={post.status === "published" ? "pill ok" : "pill quiet"}>
                  {words.status[post.status]}
                </span>
              </td>
              <td data-label={words.deleted}>
                <time dateTime={post.trashed_at}>{moment(post.trashed_at)}</time>
              </td>
              <td data-label={words.action}>
                {asking === post.key ? (
                  <div className="flex flex-wrap items-center gap-3">
                    <span>{words.asking(post.title)}</span>
                    <button
                      type="button"
                      className="btn"
                      aria-label={words.sure(post.title)}
                      onClick={() => onPurge(post)}
                    >
                      {words.sureText}
                    </button>
                  </div>
                ) : (
                  <div className="flex flex-wrap items-center gap-3">
                    <button
                      type="button"
                      className="btn"
                      aria-label={words.restore(post.title)}
                      onClick={() => onRestore(post)}
                    >
                      {words.restoreText}
                    </button>
                    <button
                      type="button"
                      className="btn"
                      aria-label={words.purge(post.title)}
                      onClick={() => setAsking(post.key)}
                    >
                      {words.purgeText}
                    </button>
                  </div>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
