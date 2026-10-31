import { type FormEvent, useState } from "react";
import type { AdminAuthor } from "../api/authors";
import { en } from "../strings/en";
import { type AuthorFields, fieldsOf } from "./edits";

type Props = { author: AdminAuthor; onSave: (fields: AuthorFields) => Promise<void> };

// One-line fields first, then the two that take several lines.
const LINES = ["name", "role", "credentials", "photo"] as const;
const BLOCKS = ["bio", "sameAs"] as const;

/** Everything an author's file says about them, ready to change. */
export function AuthorForm({ author, onSave }: Props) {
  const words = en.authors.edit.fields;
  const [fields, setFields] = useState(fieldsOf(author));
  const [busy, setBusy] = useState(false);
  const set = (key: keyof AuthorFields, value: string) => setFields({ ...fields, [key]: value });

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    await onSave(fields);
    setBusy(false);
  };

  return (
    <form onSubmit={submit}>
      <div className="pair">
        {LINES.map((key) => (
          <div className="field" key={key}>
            <label htmlFor={`edit-${key}`}>{words[key].label}</label>
            <span className="hint" id={`edit-${key}-hint`}>
              {words[key].hint}
            </span>
            <input
              id={`edit-${key}`}
              value={fields[key]}
              required={key === "name"}
              aria-describedby={`edit-${key}-hint`}
              onChange={(event) => set(key, event.target.value)}
            />
          </div>
        ))}
      </div>
      {BLOCKS.map((key) => (
        <div className="field" key={key}>
          <label htmlFor={`edit-${key}`}>{words[key].label}</label>
          <span className="hint" id={`edit-${key}-hint`}>
            {words[key].hint}
          </span>
          <textarea
            id={`edit-${key}`}
            rows={5}
            value={fields[key]}
            aria-describedby={`edit-${key}-hint`}
            onChange={(event) => set(key, event.target.value)}
          />
        </div>
      ))}
      <button type="submit" className="btn primary" disabled={busy}>
        {en.authors.edit.save}
      </button>
    </form>
  );
}
