import { type FormEvent, useState } from "react";
import type { PostRow } from "../api/rows";
import { en } from "../strings/en";
import { type BulkAction, type BulkResult, changeAll } from "./bulk";

type Props = {
  /** The posts ticked in the table. */
  ticked: PostRow[];
  /** The authors of the posts on the page, which "Set the author" offers. */
  authors: string[];
  onDone: (result: BulkResult) => void;
};

/** One form that changes every ticked post: set the author, or move them to the trash. */
export function BulkChange({ ticked, authors, onDone }: Props) {
  const words = en.bulk;
  const [kind, setKind] = useState<BulkAction["kind"]>(authors.length > 0 ? "author" : "trash");
  const [author, setAuthor] = useState(authors[0] ?? "");
  const [working, setWorking] = useState(false);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setWorking(true);
    const action: BulkAction = kind === "trash" ? { kind } : { kind, author };
    const result = await changeAll(ticked, action);
    setWorking(false);
    onDone(result);
  };

  return (
    <form className="bulk" aria-label={words.label} onSubmit={submit}>
      <p>
        <strong>{words.ticked(ticked.length)}</strong>{" "}
        <span className="muted">{words.careful}</span>
      </p>
      <div className="field">
        <label htmlFor="bulk-action">{words.action}</label>
        <select
          id="bulk-action"
          value={kind}
          onChange={(event) => setKind(event.target.value === "trash" ? "trash" : "author")}
        >
          {authors.length > 0 ? <option value="author">{words.author}</option> : null}
          <option value="trash">{words.trash}</option>
        </select>
      </div>
      {kind === "author" ? (
        <div className="field">
          <label htmlFor="bulk-author">{words.newAuthor}</label>
          <span className="hint" id="bulk-author-hint">
            {words.newAuthorHint}
          </span>
          <select
            id="bulk-author"
            aria-describedby="bulk-author-hint"
            value={author}
            onChange={(event) => setAuthor(event.target.value)}
          >
            {authors.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </div>
      ) : null}
      <button type="submit" className="btn" disabled={working}>
        {working ? words.working : words.submit(ticked.length)}
      </button>
    </form>
  );
}
