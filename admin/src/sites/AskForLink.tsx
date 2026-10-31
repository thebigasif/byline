import { type FormEvent, useState } from "react";
import { problemMessage } from "../api/posts";
import { sendLink } from "../api/sites";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";

/** Asks for a sign-in link by email: the panel needs a person, not a key. */
export function AskForLink() {
  const words = en.sites.signIn;
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  const [sent, setSent] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const to = email.trim();
    if (to === "" || busy) {
      return;
    }
    setBusy(true);
    setSent(null);
    const reply = await sendLink(to);
    setBusy(false);
    if ("data" in reply) {
      setFailed(null);
      setSent(to);
    } else {
      setFailed(problemMessage(reply.problem, en.problems.unreachable));
    }
  };

  let said: string | null = busy ? words.sending : null;
  if (sent !== null) {
    said = words.sent(sent);
  }
  return (
    <section aria-labelledby="ask-title">
      <h2 id="ask-title">{words.title}</h2>
      <p>{words.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{said}</p>
      <form className="focus" onSubmit={submit}>
        <div className="field">
          <label htmlFor="super-email">{words.label}</label>
          <span className="hint" id="super-email-hint">
            {words.hint}
          </span>
          <input
            id="super-email"
            type="email"
            autoComplete="email"
            required
            aria-describedby="super-email-hint"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
          />
        </div>
        <button type="submit" className="btn primary" disabled={busy}>
          {words.submit}
        </button>
      </form>
      <p className="muted">{words.noMail}</p>
    </section>
  );
}
