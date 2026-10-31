import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { failureMessage, isData, type Reply } from "../api/call";
import {
  changeRole,
  type Invite,
  inviteStaff,
  listStaff,
  removeStaff,
  type StaffAccount,
} from "../api/staff";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { InviteForm } from "../team/InviteForm";
import { roleName } from "../team/roles";
import { TeamTable } from "../team/TeamTable";

/** The query key the screen refreshes after any change to the team. */
export const TEAM = ["staff"];

/** The people who sign in to the admin: inviting, changing a role, removing. */
export function Team() {
  const words = en.team;
  const queries = useQueryClient();
  const listed = useQuery({ queryKey: TEAM, queryFn: listStaff }).data;
  const [invited, setInvited] = useState<string | null>(null);
  const [refused, setRefused] = useState<UserMessage | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const invite = async (person: Invite) => {
    const reply = await inviteStaff(person);
    setInvited(isData(reply) ? words.invite.invited(person.name) : null);
    setRefused(isData(reply) ? null : failureMessage(reply));
    await queries.invalidateQueries({ queryKey: TEAM });
    return isData(reply);
  };

  // A change to someone already listed: what it says when it works, or the API's words.
  const settle = async (reply: Reply<unknown>, worked: string) => {
    setDone(isData(reply) ? worked : null);
    setFailed(isData(reply) ? null : failureMessage(reply));
    await queries.invalidateQueries({ queryKey: TEAM });
  };

  const change = async (person: StaffAccount, role: string) => {
    const reply = await changeRole(person.id, role);
    await settle(reply, words.list.changed(person.name, roleName(role)));
  };

  const remove = async (person: StaffAccount) => {
    const reply = await removeStaff(person.id);
    await settle(reply, words.list.removed(person.name));
  };

  let list = <p role="status">{words.list.loading}</p>;
  if (listed !== undefined && !isData(listed)) {
    list = <Problem message={failureMessage(listed)} />;
  } else if (isData(listed)) {
    list = <TeamTable team={listed.data} onChange={change} onRemove={remove} />;
  }
  return (
    <>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      <section className="focus wide" aria-labelledby="invite-title">
        <h2 id="invite-title">{words.invite.title}</h2>
        {refused === null ? null : <Problem message={refused} />}
        <p role="status">{invited}</p>
        <InviteForm onInvite={invite} />
      </section>
      <h2 id="team-title">{words.list.title}</h2>
      <p className="muted">{words.list.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{done}</p>
      {list}
      <p>
        <Link to="/team/owner">{words.list.handOver}</Link>{" "}
        <span className="muted">{words.list.handOverHint}</span>
      </p>
    </>
  );
}
