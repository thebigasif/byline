import { type FormEvent, useState } from "react";
import { addPasskey, passkeyOptions } from "../api/account";
import { isData } from "../api/call";
import { problemMessage } from "../api/posts";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { makePasskey } from "./passkey";

type Props = { email: string };

/** The screen's focus: a passkey made by this device and added to the account. */
export function AddPasskey({ email }: Props) {
  const words = en.account.passkey;
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  // Each step stops at the first problem: the options, the device, the API.
  const add = async (label: string): Promise<UserMessage | null> => {
    const options = await passkeyOptions();
    if (!isData(options)) {
      return problemMessage(options.problem, en.problems.unreachable);
    }
    const made = await makePasskey(options.data, email, label);
    if (made === null) {
      return en.problems.noPasskey;
    }
    const added = await addPasskey({ name: label, ...made });
    return isData(added) ? null : problemMessage(added.problem, en.problems.unreachable);
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (busy) {
      return;
    }
    const label = name.trim();
    setBusy(true);
    setDone(null);
    const problem = await add(label);
    setBusy(false);
    setFailed(problem);
    if (problem === null) {
      setName("");
      setDone(words.added(label));
    }
  };

  return (
    <section className="focus" aria-labelledby="passkey-title">
      <h2 id="passkey-title">{words.title}</h2>
      <p>{words.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{busy ? words.adding : done}</p>
      <form onSubmit={submit}>
        <div className="field">
          <label htmlFor="passkey-name">{words.name}</label>
          <span className="hint" id="passkey-name-hint">
            {words.nameHint}
          </span>
          <input
            id="passkey-name"
            value={name}
            required
            aria-describedby="passkey-name-hint"
            onChange={(event) => setName(event.target.value)}
          />
        </div>
        <button type="submit" className="btn primary" disabled={busy}>
          {words.submit}
        </button>
      </form>
      <p className="muted">{words.remove}</p>
    </section>
  );
}
