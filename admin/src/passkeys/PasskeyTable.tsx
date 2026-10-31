import type { Passkey } from "../api/passkeys";
import { en } from "../strings/en";
import { PasskeyRow } from "./PasskeyRow";

type Props = {
  passkeys: Passkey[];
  onRename: (passkey: Passkey, name: string) => Promise<boolean>;
  onRemove: (passkey: Passkey) => void;
};

/** The signed-in person's passkeys: name, when each was added and last used, and what to do. */
export function PasskeyTable({ passkeys, onRename, onRemove }: Props) {
  const words = en.passkeys;
  return (
    <div className="scroll">
      <table aria-labelledby="passkeys-list-title">
        <thead>
          <tr>
            <th scope="col">{words.name}</th>
            <th scope="col">{words.added}</th>
            <th scope="col">{words.lastUsed}</th>
            <th scope="col">{words.action}</th>
          </tr>
        </thead>
        <tbody>
          {passkeys.map((passkey) => (
            <PasskeyRow
              key={passkey.id}
              passkey={passkey}
              onRename={onRename}
              onRemove={onRemove}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}
