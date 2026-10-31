import type { StaffAccount } from "../api/staff";
import { en } from "../strings/en";
import { TeamRow } from "./TeamRow";

type Props = {
  team: StaffAccount[];
  onChange: (person: StaffAccount, role: string) => void;
  onRemove: (person: StaffAccount) => void;
};

/** Everyone who signs in, each with their role. */
export function TeamTable({ team, onChange, onRemove }: Props) {
  const words = en.team.list;
  return (
    <div className="scroll">
      <table aria-labelledby="team-title">
        <thead>
          <tr>
            <th scope="col">{words.name}</th>
            <th scope="col">{words.email}</th>
            <th scope="col">{words.role}</th>
            <th scope="col">
              <span className="muted">{words.action}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {team.map((person) => (
            // Keyed by role too, so a changed role resets the row's picker.
            <TeamRow
              key={`${person.id}-${person.role}`}
              person={person}
              onChange={onChange}
              onRemove={onRemove}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}
