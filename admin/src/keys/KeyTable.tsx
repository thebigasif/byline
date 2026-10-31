import type { AdminKey } from "../api/keys";
import { day } from "../strings/dates";
import { en } from "../strings/en";

type Props = { keys: AdminKey[]; onRevoke: (key: AdminKey) => void };

// Looked up by the names the API sends, which may grow.
const SCOPES: Readonly<Record<string, string>> = en.keys.scopes;
const KINDS: Readonly<Record<string, string>> = en.keys.kinds;

/** What a key may do, in words: its scopes, its role, or reading live posts. */
function mayDo(key: AdminKey): string {
  const words = en.keys;
  if (key.kind === "staff") {
    return words.asRole(key.role ?? words.noRole);
  }
  if (key.scopes.length === 0) {
    return en.keys.scopes["content:read"];
  }
  return key.scopes.map((scope) => SCOPES[scope] ?? scope).join(", ");
}

/** The keys in use, each with what it may do and a way to switch it off. */
export function KeyTable({ keys, onRevoke }: Props) {
  const words = en.keys.list;
  return (
    <div className="scroll">
      <table aria-labelledby="keys-title">
        <thead>
          <tr>
            <th scope="col">{words.name}</th>
            <th scope="col">{words.kind}</th>
            <th scope="col">{words.may}</th>
            <th scope="col">{words.made}</th>
            <th scope="col">
              <span className="muted">{words.action}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {keys.map((key) => (
            <tr key={key.id}>
              <td>{key.name}</td>
              <td data-label={words.kind}>
                <span className="pill quiet">{KINDS[key.kind] ?? key.kind}</span>
              </td>
              <td data-label={words.may}>{mayDo(key)}</td>
              <td data-label={words.made}>{day(key.created_at)}</td>
              <td>
                <button
                  type="button"
                  className="btn"
                  aria-label={words.revokeLabel(key.name)}
                  onClick={() => onRevoke(key)}
                >
                  {words.revoke}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
