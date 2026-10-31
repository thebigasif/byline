import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { type AdminAuthor, deleteAuthor } from "../api/authors";
import { failureMessage, isData } from "../api/call";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";

type Props = { author: AdminAuthor };

/** Deletes an author's file, after saying what happens to their posts. */
export function DeleteAuthor({ author }: Props) {
  const words = en.authors.edit.remove;
  const navigate = useNavigate();
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const remove = async () => {
    const reply = await deleteAuthor(author.id, author.updated_at);
    if (isData(reply)) {
      await navigate({ to: "/authors" });
    } else {
      setFailed(failureMessage(reply));
    }
  };

  return (
    <section aria-labelledby="delete-author-title">
      <h2 id="delete-author-title">{words.title}</h2>
      <p className="muted">{words.lede(author.name)}</p>
      {failed === null ? null : <Problem message={failed} />}
      <button type="button" className="btn" onClick={remove}>
        {words.submit(author.name)}
      </button>
    </section>
  );
}
