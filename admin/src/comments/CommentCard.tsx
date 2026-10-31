import type { AdminComment } from "../api/comments";
import { day } from "../strings/dates";
import { en } from "../strings/en";
import { postName } from "./postName";

type Props = {
  comment: AdminComment;
  /** Whether this is the next comment to decide on, drawn as the screen's focus. */
  first: boolean;
  onApprove: (comment: AdminComment) => void;
  onDelete: (comment: AdminComment) => void;
};

/** One comment: who wrote it on which post, what it says, and what to do with it. */
export function CommentCard({ comment, first, onApprove, onDelete }: Props) {
  const words = en.comments;
  const name = comment.author_name;
  const byStaff = comment.author_kind === "staff";
  return (
    <li className={first ? "focus wide" : "card"}>
      <p className="flex flex-wrap items-baseline gap-2">
        <strong>{name}</strong>
        {byStaff ? <span className="pill quiet">{words.staff}</span> : null}
        <span className="muted">{words.on(postName(comment.post), day(comment.created_at))}</span>
      </p>
      <p className="said">{comment.text}</p>
      <div className="flex flex-wrap items-center gap-3">
        {comment.status === "approved" ? null : (
          <button
            type="button"
            className={first ? "btn primary" : "btn"}
            aria-label={words.approveLabel(name)}
            onClick={() => onApprove(comment)}
          >
            {words.approve}
          </button>
        )}
        <button
          type="button"
          className="btn"
          aria-label={words.removeLabel(name)}
          onClick={() => onDelete(comment)}
        >
          {words.remove}
        </button>
      </div>
    </li>
  );
}
