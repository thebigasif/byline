import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { problemMessage } from "../api/posts";
import { isData, listMisses, type Miss, redirectMiss } from "../api/redirects";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { KEPT } from "./KeptRedirects";
import { MissTable } from "./MissTable";

const MISSES = ["misses"];

/** The 404 log: addresses that found nothing, each sent to its nearest page in one step. */
export function MissLog() {
  const words = en.redirects.misses;
  const queries = useQueryClient();
  const misses = useQuery({ queryKey: MISSES, queryFn: listMisses }).data;
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const send = async (miss: Miss, to: string) => {
    const reply = await redirectMiss(miss.path, to);
    if (isData(reply)) {
      setFailed(null);
      setDone(words.sent(miss.path, to));
      await Promise.all([
        queries.invalidateQueries({ queryKey: MISSES }),
        queries.invalidateQueries({ queryKey: KEPT }),
      ]);
    } else {
      setDone(null);
      setFailed(problemMessage(reply.problem, en.problems.unreachable));
    }
  };

  let list = <p role="status">{words.loading}</p>;
  if (misses !== undefined && !isData(misses)) {
    list = <Problem message={problemMessage(misses.problem, en.problems.unreachable)} />;
  } else if (isData(misses) && misses.data.length === 0) {
    list = <p role="status">{words.none}</p>;
  } else if (isData(misses)) {
    list = <MissTable misses={misses.data} onSend={send} />;
  }
  return (
    <section aria-labelledby="misses-title">
      <h2 id="misses-title">{words.title}</h2>
      <p className="muted">{words.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{done}</p>
      {list}
    </section>
  );
}
