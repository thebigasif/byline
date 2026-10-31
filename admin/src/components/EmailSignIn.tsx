import { useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { signInWithEmail } from "../api/account";
import { isData } from "../api/call";
import { problemMessage } from "../api/posts";
import { saveCsrf } from "../auth/session";
import { en, type UserMessage } from "../strings/en";
import { Problem } from "./Problem";

/** Signing in with an account's email and password, for someone the owner invited. */
export function EmailSignIn() {
  const words = en.signIn.email;
  const navigate = useNavigate();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [checking, setChecking] = useState(false);
  const [problem, setProblem] = useState<UserMessage | null>(null);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setChecking(true);
    const reply = await signInWithEmail(email.trim(), password);
    setChecking(false);
    if (isData(reply)) {
      saveCsrf(reply.data.csrf);
      await navigate({ to: "/" });
      return;
    }
    setPassword("");
    setProblem(problemMessage(reply.problem, en.problems.unreachable));
  };

  return (
    <section className="mt-8 max-w-160" aria-labelledby="email-title">
      <h2 id="email-title">{words.title}</h2>
      <p className="hint">{words.lede}</p>
      {problem === null ? null : <Problem message={problem} />}
      <form onSubmit={submit}>
        <div className="field">
          <label htmlFor="email">{words.address}</label>
          <input
            id="email"
            type="email"
            autoComplete="email"
            required
            value={email}
            onChange={(event) => setEmail(event.target.value)}
          />
        </div>
        <div className="field">
          <label htmlFor="password">{words.password}</label>
          <input
            id="password"
            type="password"
            autoComplete="current-password"
            required
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
        </div>
        <button type="submit" className="btn" disabled={checking}>
          {checking ? words.checking : words.submit}
        </button>
      </form>
    </section>
  );
}
