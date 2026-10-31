import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { listPasskeys, type Passkey, removePasskey, renamePasskey } from "../api/passkeys";
import { problemMessage } from "../api/posts";
import { isData, type Reply } from "../api/redirects";
import { Problem } from "../components/Problem";
import { AddPasskey } from "../passkeys/AddPasskey";
import { PasskeyTable } from "../passkeys/PasskeyTable";
import { en, type UserMessage } from "../strings/en";

/** The query key the screen refreshes after a passkey is added, renamed or removed. */
export const PASSKEYS = ["passkeys"];

/** The signed-in person's own passkeys, to add, name and remove. */
export function Passkeys() {
  const words = en.passkeys;
  const queries = useQueryClient();
  const passkeys = useQuery({ queryKey: PASSKEYS, queryFn: listPasskeys }).data;
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  // Says what happened, or why not, then reads the passkeys again.
  const settle = async (reply: Reply<unknown>, said: string) => {
    const worked = isData(reply);
    setDone(worked ? said : null);
    setFailed(worked ? null : problemMessage(reply.problem, en.problems.unreachable));
    await queries.invalidateQueries({ queryKey: PASSKEYS });
    return worked;
  };
  const rename = async (passkey: Passkey, name: string) => {
    if (name === "") {
      setDone(null);
      setFailed(words.problems.blankName);
      return false;
    }
    return settle(await renamePasskey(passkey.id, name), words.renamed(name));
  };
  const remove = async (passkey: Passkey) => {
    await settle(await removePasskey(passkey.id), words.removed(passkey.name));
  };
  const added = async (name: string | null, problem: UserMessage | null) => {
    setDone(name === null ? null : words.madeOne(name));
    setFailed(problem);
    await queries.invalidateQueries({ queryKey: PASSKEYS });
  };

  let list = <p role="status">{words.loading}</p>;
  if (passkeys !== undefined && !isData(passkeys)) {
    // Only a session belongs to a person; a tab signed in with a key has none.
    const problem =
      passkeys.status === 401
        ? words.problems.noSession
        : problemMessage(passkeys.problem, en.problems.unreachable);
    return (
      <>
        <h1>{words.title}</h1>
        <p className="lede">{words.lede}</p>
        <Problem message={problem} />
      </>
    );
  }
  if (isData(passkeys)) {
    const count = passkeys.data.length;
    list = (
      <>
        <p role="status">{count === 0 ? words.none : words.count(count)}</p>
        {count === 0 ? null : (
          <PasskeyTable passkeys={passkeys.data} onRename={rename} onRemove={remove} />
        )}
      </>
    );
  }
  return (
    <>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{done}</p>
      <AddPasskey onDone={added} />
      <h2 id="passkeys-list-title">{words.listTitle}</h2>
      <p className="muted">{words.listHint}</p>
      {list}
    </>
  );
}
