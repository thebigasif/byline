import { useState } from "react";
import type { StaffAccount } from "../api/staff";
import { en } from "../strings/en";
import { GIVEN, roleName } from "./roles";

type Props = {
  person: StaffAccount;
  onChange: (person: StaffAccount, role: string) => void;
  onRemove: (person: StaffAccount) => void;
};

/** One person on the team: a role to change and a way to remove them, but not the owner. */
export function TeamRow({ person, onChange, onRemove }: Props) {
  const words = en.team.list;
  const [role, setRole] = useState(person.role);
  const picker = `role-${person.id}`;
  if (person.role === "owner") {
    return (
      <tr>
        <td>{person.name}</td>
        <td data-label={words.email}>{person.email}</td>
        <td data-label={words.role}>{roleName(person.role)}</td>
        <td>
          <span className="muted">{words.owner}</span>
        </td>
      </tr>
    );
  }
  return (
    <tr>
      <td>{person.name}</td>
      <td data-label={words.email}>{person.email}</td>
      <td data-label={words.role}>
        <div className="flex flex-wrap items-center gap-3">
          <label className="hint" htmlFor={picker}>
            {words.roleFor(person.name)}
          </label>
          <select id={picker} value={role} onChange={(event) => setRole(event.target.value)}>
            {GIVEN.map((each) => (
              <option key={each} value={each}>
                {roleName(each)}
              </option>
            ))}
          </select>
          <button
            type="button"
            className="btn"
            aria-label={words.changeLabel(person.name)}
            disabled={role === person.role}
            onClick={() => onChange(person, role)}
          >
            {words.change}
          </button>
        </div>
      </td>
      <td>
        <button
          type="button"
          className="btn"
          aria-label={words.removeLabel(person.name)}
          onClick={() => onRemove(person)}
        >
          {words.remove}
        </button>
      </td>
    </tr>
  );
}
