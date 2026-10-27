import { useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { checkKey, type KeyCheck } from "../auth/checkKey";
import { saveKey } from "../auth/session";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";

const MESSAGES: Record<Exclude<KeyCheck, "accepted">, UserMessage> = {
  rejected: en.problems.rejected,
  "lacks-scope": en.problems.lacksScope,
  unreachable: en.problems.unreachable,
};

/** Asks for a staff token and keeps it once the blog accepts it. */
export function SignIn() {
  const navigate = useNavigate();
  const [key, setKey] = useState("");
  const [checking, setChecking] = useState(false);
  const [problem, setProblem] = useState<UserMessage | null>(null);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setChecking(true);
    const answer = await checkKey(key.trim());
    setChecking(false);
    if (answer === "accepted") {
      saveKey(key.trim());
      await navigate({ to: "/" });
      return;
    }
    setProblem(MESSAGES[answer]);
  };

  return (
    <main id="main" className="page">
      <h1>{en.signIn.title}</h1>
      <p className="lede">{en.signIn.lede}</p>
      {problem === null ? null : <Problem message={problem} />}
      <form className="focus" onSubmit={submit}>
        <div className="field">
          <label htmlFor="key">{en.signIn.label}</label>
          <span className="hint" id="key-hint">
            {en.signIn.hint}
          </span>
          <input
            id="key"
            type="password"
            autoComplete="current-password"
            spellCheck={false}
            required
            aria-describedby="key-hint"
            value={key}
            onChange={(event) => setKey(event.target.value)}
          />
        </div>
        <button type="submit" className="btn primary" disabled={checking}>
          {checking ? en.signIn.checking : en.signIn.submit}
        </button>
      </form>
    </main>
  );
}
