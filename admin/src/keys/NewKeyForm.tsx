import { type FormEvent, useState } from "react";
import type { NewKey, NewKeyKind } from "../api/keys";
import { en } from "../strings/en";

type Props = { onCreate: (key: NewKey) => Promise<boolean> };

const KINDS: NewKeyKind[] = ["content", "integration"];
// What an integration key may be given; `content:read` is what a content key does.
const SCOPES = [
  "posts:write",
  "posts:publish",
  "media:write",
  "settings:write",
  "webhooks:manage",
  "subscribers:write",
  "admin:all",
] as const;

/** A new key's name, kind and, for an integration key, what it may do. */
export function NewKeyForm({ onCreate }: Props) {
  const words = en.keys.create;
  const [name, setName] = useState("");
  const [kind, setKind] = useState<NewKeyKind>("content");
  const [scopes, setScopes] = useState<string[]>(["posts:write"]);
  const [busy, setBusy] = useState(false);
  const toggle = (scope: string, on: boolean) =>
    setScopes(on ? [...scopes, scope] : scopes.filter((each) => each !== scope));

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    const made = await onCreate(
      kind === "content" ? { name: name.trim(), kind } : { name: name.trim(), kind, scopes },
    );
    setBusy(false);
    if (made) {
      setName("");
    }
  };

  return (
    <form onSubmit={submit}>
      <div className="field">
        <label htmlFor="key-name">{words.name}</label>
        <span className="hint" id="key-name-hint">
          {words.nameHint}
        </span>
        <input
          id="key-name"
          value={name}
          required
          aria-describedby="key-name-hint"
          onChange={(event) => setName(event.target.value)}
        />
      </div>
      <fieldset>
        <legend>{words.kind}</legend>
        {KINDS.map((each) => (
          <label className="choice" key={each}>
            <input
              type="radio"
              name="key-kind"
              checked={kind === each}
              onChange={() => setKind(each)}
            />
            <span>
              <strong>{en.keys.kinds[each]}</strong>
              <span className="hint">{words.kindHints[each]}</span>
            </span>
          </label>
        ))}
      </fieldset>
      {kind === "integration" ? (
        <fieldset>
          <legend>{words.scopes}</legend>
          {SCOPES.map((scope) => (
            <label className="choice" key={scope}>
              <input
                type="checkbox"
                checked={scopes.includes(scope)}
                onChange={(event) => toggle(scope, event.target.checked)}
              />
              <span>{en.keys.scopes[scope]}</span>
            </label>
          ))}
        </fieldset>
      ) : null}
      <p className="muted">{words.once}</p>
      <button type="submit" className="btn primary" disabled={busy}>
        {words.submit}
      </button>
    </form>
  );
}
