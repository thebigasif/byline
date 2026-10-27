import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { problemMessage } from "../api/posts";
import { isData, listRedirects, removeRedirect, type StoredRedirect } from "../api/redirects";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { KeptTable } from "./KeptTable";

/** The query key every part of the screen refreshes after a redirect is stored or removed. */
export const KEPT = ["redirects"];

/** The redirects Khalid keeps in its database, each with a way to remove it. */
export function KeptRedirects() {
  const words = en.redirects.kept;
  const queries = useQueryClient();
  const kept = useQuery({ queryKey: KEPT, queryFn: listRedirects }).data;
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const remove = async (redirect: StoredRedirect) => {
    const reply = await removeRedirect(redirect.id);
    if (isData(reply)) {
      setFailed(null);
      setDone(words.removed(redirect.from));
      await queries.invalidateQueries({ queryKey: KEPT });
    } else {
      setDone(null);
      setFailed(problemMessage(reply.problem, en.problems.unreachable));
    }
  };

  let list = <p role="status">{words.loading}</p>;
  if (kept !== undefined && !isData(kept)) {
    list = <Problem message={problemMessage(kept.problem, en.problems.unreachable)} />;
  } else if (isData(kept) && kept.data.length === 0) {
    list = <p role="status">{words.none}</p>;
  } else if (isData(kept)) {
    list = <KeptTable redirects={kept.data} onRemove={remove} />;
  }
  return (
    <section aria-labelledby="kept-title">
      <h2 id="kept-title">{words.title}</h2>
      <p className="muted">{words.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{done}</p>
      {list}
      <p className="muted">{words.after}</p>
    </section>
  );
}
