import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { problemMessage } from "../api/posts";
import { listTrash, purgeTrashed, restoreTrashed, type TrashedPost } from "../api/records";
import { isData, type Reply } from "../api/redirects";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { TrashTable } from "../trash/TrashTable";

/** The query key the screen refreshes after a post comes back or goes for good. */
export const TRASH = ["trash"];

/** The deleted posts, each with a way to put it back or delete it for good. */
export function Trash() {
  const words = en.trash;
  const queries = useQueryClient();
  const trash = useQuery({ queryKey: TRASH, queryFn: listTrash }).data;
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  // Says what happened, or why not, then reads the trash again.
  const settle = async (reply: Reply<unknown>, said: string) => {
    if (isData(reply)) {
      setFailed(null);
      setDone(said);
    } else {
      setDone(null);
      setFailed(problemMessage(reply.problem, en.problems.unreachable));
    }
    await queries.invalidateQueries({ queryKey: TRASH });
  };
  const restore = async (post: TrashedPost) => {
    await settle(await restoreTrashed(post.key), words.restored(post.title));
  };
  const purge = async (post: TrashedPost) => {
    await settle(await purgeTrashed(post.key), words.purged(post.title));
  };

  let list = <p role="status">{words.loading}</p>;
  if (trash !== undefined && !isData(trash)) {
    list = <Problem message={problemMessage(trash.problem, en.problems.unreachable)} />;
  } else if (isData(trash) && trash.data.length === 0) {
    list = <p role="status">{words.none}</p>;
  } else if (isData(trash)) {
    list = (
      <>
        <p role="status">{words.count(trash.data.length)}</p>
        <TrashTable trash={trash.data} onRestore={restore} onPurge={purge} />
      </>
    );
  }
  return (
    <>
      <h1 id="trash-title">{words.title}</h1>
      <p className="lede">{words.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{done}</p>
      {list}
      <h2>{words.beforeTitle}</h2>
      <p>{words.before}</p>
    </>
  );
}
