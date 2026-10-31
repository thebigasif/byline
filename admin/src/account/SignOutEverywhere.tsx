import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { signOutEverywhere } from "../api/account";
import { isData } from "../api/call";
import { problemMessage } from "../api/posts";
import { forgetKey } from "../auth/session";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";

/** Ends every session of the account, this tab's too, for a lost laptop. */
export function SignOutEverywhere() {
  const words = en.account.everywhere;
  const navigate = useNavigate();
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const end = async () => {
    const reply = await signOutEverywhere();
    if (!isData(reply)) {
      setFailed(problemMessage(reply.problem, en.problems.unreachable));
      return;
    }
    forgetKey();
    await navigate({ to: "/sign-in" });
  };

  return (
    <section className="max-w-160" aria-labelledby="everywhere-title">
      <h2 id="everywhere-title">{words.title}</h2>
      <p className="muted">{words.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <button type="button" className="btn" onClick={end}>
        {words.submit}
      </button>
    </section>
  );
}
