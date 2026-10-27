import { useState } from "react";
import type { AdminPost } from "../api/posts";
import { en } from "../strings/en";

type Props = { post: AdminPost };

/** The words of a post a writer changes in the editor. */
export type Draft = { title: string; markdown: string };

/** The title and text of one post. */
export function EditorForm({ post }: Props) {
  const [draft, setDraft] = useState<Draft>({ title: post.title, markdown: post.markdown });

  return (
    <>
      <h1 className="sr-only">{`${en.editor.heading} ${draft.title}`}</h1>
      <label className="sr-only" htmlFor="title">
        {en.editor.title}
      </label>
      <input
        id="title"
        className="title"
        value={draft.title}
        onChange={(event) => setDraft((current) => ({ ...current, title: event.target.value }))}
      />
      <textarea
        className="source"
        aria-label={en.editor.text}
        value={draft.markdown}
        onChange={(event) => setDraft((current) => ({ ...current, markdown: event.target.value }))}
        rows={24}
        spellCheck
      />
    </>
  );
}
