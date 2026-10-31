import { type FormEvent, useState } from "react";
import type { StaffAccount } from "../api/staff";
import { en } from "../strings/en";

type Props = { others: StaffAccount[]; onHandOver: (person: StaffAccount) => void };

/** Picks the new owner, and asks for the consequence to be understood before it hands over. */
export function HandOverForm({ others, onHandOver }: Props) {
  const words = en.team.handover;
  const [picked, setPicked] = useState(others[0]?.id ?? 0);
  const [understood, setUnderstood] = useState(false);
  const person = others.find((each) => each.id === picked) ?? others[0];
  if (person === undefined) {
    return <p>{words.nobody}</p>;
  }

  const submit = (event: FormEvent) => {
    event.preventDefault();
    onHandOver(person);
  };

  return (
    <form onSubmit={submit}>
      <div className="field">
        <label htmlFor="next-owner">{words.next}</label>
        <span className="hint" id="next-owner-hint">
          {words.nextHint}
        </span>
        <select
          id="next-owner"
          aria-describedby="next-owner-hint"
          value={person.id}
          onChange={(event) => setPicked(Number(event.target.value))}
        >
          {others.map((each) => (
            <option key={each.id} value={each.id}>
              {`${each.name}, ${each.email}`}
            </option>
          ))}
        </select>
      </div>
      <label className="choice">
        <input
          type="checkbox"
          checked={understood}
          onChange={(event) => setUnderstood(event.target.checked)}
        />
        <span>{words.understand}</span>
      </label>
      <button type="submit" className="btn primary" disabled={!understood}>
        {words.submit(person.name)}
      </button>
    </form>
  );
}
