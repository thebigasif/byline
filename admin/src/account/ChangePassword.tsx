import { type FormEvent, useState } from "react";
import { changePassword } from "../api/account";
import { isData } from "../api/call";
import { problemMessage } from "../api/posts";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";

/** A new password, which signs the account out on every other device. */
export function ChangePassword() {
  const words = en.account.password;
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    const reply = await changePassword(current === "" ? null : current, next);
    setBusy(false);
    setDone(isData(reply));
    setFailed(isData(reply) ? null : problemMessage(reply.problem, en.problems.unreachable));
    setCurrent("");
    setNext("");
  };

  return (
    <section className="max-w-160" aria-labelledby="password-title">
      <h2 id="password-title">{words.title}</h2>
      <p className="muted">{words.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{done ? words.changed : null}</p>
      <form onSubmit={submit}>
        <div className="field">
          <label htmlFor="current-password">{words.current}</label>
          <span className="hint" id="current-password-hint">
            {words.currentHint}
          </span>
          <input
            id="current-password"
            type="password"
            autoComplete="current-password"
            aria-describedby="current-password-hint"
            value={current}
            onChange={(event) => setCurrent(event.target.value)}
          />
        </div>
        <div className="field">
          <label htmlFor="new-password">{words.next}</label>
          <span className="hint" id="new-password-hint">
            {words.nextHint}
          </span>
          <input
            id="new-password"
            type="password"
            autoComplete="new-password"
            minLength={12}
            required
            aria-describedby="new-password-hint"
            value={next}
            onChange={(event) => setNext(event.target.value)}
          />
        </div>
        <button type="submit" className="btn" disabled={busy}>
          {words.submit}
        </button>
      </form>
    </section>
  );
}
