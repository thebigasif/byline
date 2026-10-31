import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { problemMessage } from "../api/posts";
import { isData, type Reply } from "../api/redirects";
import { connectSerpdad, disconnectSerpdad, readSerpdad, type SerpdadState } from "../api/serpdad";
import { Problem } from "../components/Problem";
import { Connection } from "../serpdad/Connection";
import { en, type UserMessage } from "../strings/en";

const SERPDAD = ["serpdad"];

/** Connect to SerpDad: make the key SerpDad sends drafts with, and see what arrived. */
export function Serpdad() {
  const words = en.serpdad;
  const queries = useQueryClient();
  const state = useQuery({ queryKey: SERPDAD, queryFn: readSerpdad }).data;
  // The key is in the answer that made it and nowhere else, so it lives only here.
  const [secret, setSecret] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);
  const [busy, setBusy] = useState(false);

  const act = async <T,>(call: () => Promise<Reply<T>>, after: (data: T) => void) => {
    setBusy(true);
    const reply = await call();
    setBusy(false);
    if (isData(reply)) {
      setFailed(null);
      after(reply.data);
      await queries.invalidateQueries({ queryKey: SERPDAD });
    } else {
      setFailed(problemMessage(reply.problem, en.problems.unreachable));
    }
  };
  const make = async () => {
    await act(connectSerpdad, (made: SerpdadState) => {
      setDone(null);
      setSecret(made.connection?.key ?? null);
    });
  };
  const off = async () => {
    await act(disconnectSerpdad, () => {
      setSecret(null);
      setDone(words.change.turnedOff);
    });
  };

  let body = <p role="status">{words.loading}</p>;
  if (state !== undefined && !isData(state)) {
    body = <Problem message={problemMessage(state.problem, en.problems.unreachable)} />;
  } else if (isData(state)) {
    const shown = { state: state.data, secret, busy };
    body = <Connection {...shown} onMake={() => void make()} onOff={() => void off()} />;
  }
  return (
    <>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{done}</p>
      {body}
    </>
  );
}
