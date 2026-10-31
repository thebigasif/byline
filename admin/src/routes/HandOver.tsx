import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { failureMessage, isData } from "../api/call";
import { handOver, listStaff, type StaffAccount } from "../api/staff";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { HandOverForm } from "../team/HandOverForm";
import { TEAM } from "./Team";

/** Hands the site to someone else on the team, after saying what changes. */
export function HandOver() {
  const words = en.team.handover;
  const queries = useQueryClient();
  const listed = useQuery({ queryKey: TEAM, queryFn: listStaff }).data;
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const handTo = async (person: StaffAccount) => {
    const reply = await handOver(person.id);
    setDone(isData(reply) ? words.done(person.name) : null);
    setFailed(isData(reply) ? null : failureMessage(reply));
    await queries.invalidateQueries({ queryKey: TEAM });
  };

  let form = <p role="status">{words.loading}</p>;
  if (listed !== undefined && !isData(listed)) {
    form = <Problem message={failureMessage(listed)} />;
  } else if (isData(listed)) {
    const others = listed.data.filter((person) => person.role !== "owner");
    form = <HandOverForm others={others} onHandOver={handTo} />;
  }
  return (
    <>
      <p>
        <Link to="/team">{words.back}</Link>
      </p>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      <h2 id="changes-title">{words.changes}</h2>
      <ul className="list-disc pl-6" aria-labelledby="changes-title">
        {words.consequences.map((consequence) => <li key={consequence}>{consequence}</li>)}
      </ul>
      <section className="focus" aria-labelledby="pick-title">
        <h2 id="pick-title">{words.pick}</h2>
        {failed === null ? null : <Problem message={failed} />}
        <p role="status">{done}</p>
        {done === null ? form : null}
      </section>
    </>
  );
}
