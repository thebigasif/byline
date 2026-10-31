import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { problemMessage } from "../../api/posts";
import { isData } from "../../api/redirects";
import { createSnippet, snippetSlug } from "../../api/snippets";
import { Problem } from "../../components/Problem";
import { en, type UserMessage } from "../../strings/en";

type Props = {
  /** The selected blocks, in markdown. */
  markdown: string;
  /** Called with the new snippet's slug once it is saved. */
  onSaved: (slug: string) => void;
  onCancel: () => void;
};

/** Saves the selected blocks as a new snippet, under a name the writer gives. */
export function SaveSnippet({ markdown, onSaved, onCancel }: Props) {
  const [name, setName] = useState("");
  const [problem, setProblem] = useState<UserMessage | null>(null);
  const [busy, setBusy] = useState(false);
  const field = useRef<HTMLInputElement>(null);
  const queries = useQueryClient();
  useEffect(() => field.current?.focus(), []);
  const save = async () => {
    const slug = snippetSlug(name);
    if (slug === "") {
      setProblem(en.snippets.noName);
      return;
    }
    setBusy(true);
    const reply = await createSnippet({ slug, title: name.trim(), markdown });
    setBusy(false);
    if (isData(reply)) {
      await queries.invalidateQueries({ queryKey: ["snippets"] });
      onSaved(slug);
    } else if (reply.status === 409) {
      setProblem(en.snippets.taken);
    } else {
      setProblem(problemMessage(reply.problem, en.problems.unreachable));
    }
  };
  return (
    <form
      className="focus block-form"
      aria-labelledby="snippet-form-title"
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <h2 id="snippet-form-title">{en.snippets.title}</h2>
      <p className="hint">{en.snippets.hint}</p>
      <div className="field">
        <label htmlFor="snippet-name">{en.snippets.name}</label>
        <input
          id="snippet-name"
          ref={field}
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
      </div>
      {problem === null ? null : <Problem message={problem} />}
      <div className="flex flex-wrap gap-3">
        <button type="submit" className="btn primary" disabled={busy}>
          {en.snippets.save}
        </button>
        <button type="button" className="btn" onClick={onCancel}>
          {en.details.cancel}
        </button>
      </div>
    </form>
  );
}
