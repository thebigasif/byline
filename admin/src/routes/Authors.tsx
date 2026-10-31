import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { createAuthor, listAuthors, type NewAuthor } from "../api/authors";
import { failureMessage, isData } from "../api/call";
import { AuthorTable } from "../authors/AuthorTable";
import { NewAuthorForm } from "../authors/NewAuthorForm";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";

/** The query key the screen refreshes after an author is added, changed or deleted. */
export const AUTHORS = ["authors"];

/** The people named on posts, and adding one. */
export function Authors() {
  const words = en.authors;
  const queries = useQueryClient();
  const listed = useQuery({ queryKey: AUTHORS, queryFn: listAuthors }).data;
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const add = async (author: NewAuthor) => {
    const reply = await createAuthor(author);
    if (!isData(reply)) {
      setDone(null);
      setFailed(failureMessage(reply));
      return false;
    }
    setFailed(null);
    setDone(words.add.added(author.name));
    await queries.invalidateQueries({ queryKey: AUTHORS });
    return true;
  };

  let list = <p role="status">{words.list.loading}</p>;
  if (listed !== undefined && !isData(listed)) {
    list = <Problem message={failureMessage(listed)} />;
  } else if (isData(listed) && listed.data.length === 0) {
    list = <p role="status">{words.list.none}</p>;
  } else if (isData(listed)) {
    list = <AuthorTable authors={listed.data} />;
  }
  return (
    <>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      <section className="focus wide" aria-labelledby="new-author-title">
        <h2 id="new-author-title">{words.add.title}</h2>
        {failed === null ? null : <Problem message={failed} />}
        <p role="status">{done}</p>
        <NewAuthorForm onAdd={add} />
      </section>
      <h2 id="authors-title">{words.list.title}</h2>
      {list}
    </>
  );
}
