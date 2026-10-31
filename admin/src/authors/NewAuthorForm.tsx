import { type FormEvent, useState } from "react";
import type { NewAuthor } from "../api/authors";
import { en } from "../strings/en";

type Props = { onAdd: (author: NewAuthor) => Promise<boolean> };

// Each field: its key in the new author, and whether it must be filled in.
const FIELDS = [
  ["name", true],
  ["slug", true],
  ["role", false],
  ["language", false],
] as const;

/** The screen's focus: a new author's name, short name, role and language. */
export function NewAuthorForm({ onAdd }: Props) {
  const words = en.authors.add;
  const [values, setValues] = useState({ name: "", slug: "", role: "", language: "" });
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    const role = values.role.trim();
    const language = values.language.trim();
    const added = await onAdd({
      name: values.name.trim(),
      slug: values.slug.trim(),
      role: role === "" ? null : role,
      language: language === "" ? null : language,
    });
    setBusy(false);
    if (added) {
      setValues({ name: "", slug: "", role: "", language: "" });
    }
  };

  return (
    <form onSubmit={submit}>
      <div className="pair">
        {FIELDS.map(([key, required]) => (
          <div className="field" key={key}>
            <label htmlFor={`author-${key}`}>{words.fields[key].label}</label>
            <span className="hint" id={`author-${key}-hint`}>
              {words.fields[key].hint}
            </span>
            <input
              id={`author-${key}`}
              value={values[key]}
              required={required}
              aria-describedby={`author-${key}-hint`}
              onChange={(event) => setValues({ ...values, [key]: event.target.value })}
            />
          </div>
        ))}
      </div>
      <button type="submit" className="btn primary" disabled={busy}>
        {words.submit}
      </button>
    </form>
  );
}
