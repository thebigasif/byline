import { useQuery, useQueryClient } from "@tanstack/react-query";
import { getRouteApi, Link } from "@tanstack/react-router";
import { useState } from "react";
import { type AdminAuthor, editAuthor, readAuthor } from "../api/authors";
import { failureMessage, isData } from "../api/call";
import { AuthorForm } from "../authors/AuthorForm";
import { DeleteAuthor } from "../authors/DeleteAuthor";
import { type AuthorFields, changes } from "../authors/edits";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { languageName } from "../strings/languages";
import { AUTHORS } from "./Authors";

const route = getRouteApi("/signed-in/authors/$id");

/** One author's page: what their file says, saved field by field, and deleting them. */
export function AuthorPage() {
  const words = en.authors.edit;
  const { id } = route.useParams();
  const queries = useQueryClient();
  const read = useQuery({ queryKey: [...AUTHORS, id], queryFn: () => readAuthor(id) }).data;
  const [said, setSaid] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const save = async (author: AdminAuthor, fields: AuthorFields) => {
    const edit = changes(author, fields);
    if (edit === null) {
      setSaid(words.unchanged);
      return;
    }
    const reply = await editAuthor(author.id, edit);
    setSaid(isData(reply) ? words.saved : null);
    setFailed(isData(reply) ? null : failureMessage(reply));
    await queries.invalidateQueries({ queryKey: AUTHORS });
  };

  const author = isData(read) ? read.data[0] : undefined;
  let shown = <p role="status">{words.loading}</p>;
  if (read !== undefined && !isData(read)) {
    shown = <Problem message={failureMessage(read)} />;
  } else if (author !== undefined) {
    shown = (
      <>
        <p className="lede">{words.lede(author.slug, languageName(author.language))}</p>
        {failed === null ? null : <Problem message={failed} />}
        <p role="status">{said}</p>
        <AuthorForm author={author} onSave={(fields) => save(author, fields)} />
        <DeleteAuthor author={author} />
      </>
    );
  }
  return (
    <>
      <p>
        <Link to="/authors">{words.back}</Link>
      </p>
      <h1>{author?.name ?? words.title}</h1>
      {shown}
    </>
  );
}
