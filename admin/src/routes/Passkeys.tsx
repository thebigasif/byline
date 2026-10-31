import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { listPasskeys, type Passkey, removePasskey } from "../api/passkeys";
import { problemMessage } from "../api/posts";
import { isData, type Reply } from "../api/redirects";
import { Problem } from "../components/Problem";
import { PasskeyTable } from "../passkeys/PasskeyTable";
import { en, type UserMessage } from "../strings/en";

/** The query key the screen refreshes after a passkey is added, renamed or removed. */
export const PASSKEYS = ["passkeys"];

/** The signed-in person's own passkeys, to see and remove. */
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
  const remove = async (passkey: Passkey) => {
    await settle(await removePasskey(passkey.id), words.removed(passkey.name));
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
          <PasskeyTable passkeys={passkeys.data} onRemove={remove} />
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
      <h2 id="passkeys-list-title">{words.listTitle}</h2>
      <p className="muted">{words.listHint}</p>
      {list}
    </>
  );
}
