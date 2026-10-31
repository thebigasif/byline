import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { failureMessage, isData } from "../api/call";
import { type AdminKey, listKeys, revokeKey } from "../api/keys";
import { Problem } from "../components/Problem";
import { KeyTable } from "../keys/KeyTable";
import { en, type UserMessage } from "../strings/en";

/** The query key the screen refreshes after a key is made or switched off. */
export const KEYS = ["keys"];

/** The keys other programs use to reach the blog, and switching one off. */
export function Keys() {
  const words = en.keys;
  const queries = useQueryClient();
  const listed = useQuery({ queryKey: KEYS, queryFn: listKeys }).data;
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const revoke = async (key: AdminKey) => {
    const reply = await revokeKey(key.id);
    setDone(isData(reply) ? words.list.revoked(key.name) : null);
    setFailed(isData(reply) ? null : failureMessage(reply));
    await queries.invalidateQueries({ queryKey: KEYS });
  };

  let list = <p role="status">{words.list.loading}</p>;
  if (listed !== undefined && !isData(listed)) {
    list = <Problem message={failureMessage(listed)} />;
  } else if (isData(listed)) {
    list = <KeyTable keys={listed.data} onRevoke={revoke} />;
  }
  return (
    <>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      <h2 id="keys-title">{words.list.title}</h2>
      <p className="muted">{words.list.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{done}</p>
      {list}
    </>
  );
}
