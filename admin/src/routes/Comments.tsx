import { useState } from "react";
import type { CommentStatus } from "../api/comments";
import { CommentQueue } from "../comments/CommentQueue";
import { en } from "../strings/en";

const STATUSES: CommentStatus[] = ["pending", "approved", "spam", "trash"];

/** Readers' comments: the ones waiting for a yes first, the rest one switch away. */
export function Comments() {
  const words = en.comments;
  const [status, setStatus] = useState<CommentStatus>("pending");
  return (
    <>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      <fieldset className="switch spaced" aria-label={words.which}>
        {STATUSES.map((each) => (
          <button
            key={each}
            type="button"
            aria-pressed={each === status}
            onClick={() => setStatus(each)}
          >
            {words.statuses[each]}
          </button>
        ))}
      </fieldset>
      <CommentQueue status={status} />
      <p className="muted">{words.after}</p>
    </>
  );
}
