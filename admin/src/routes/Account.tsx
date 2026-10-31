import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { ChangePassword } from "../account/ChangePassword";
import { SignOutEverywhere } from "../account/SignOutEverywhere";
import { readSession } from "../api/account";
import { isData } from "../api/call";
import { Problem } from "../components/Problem";
import { en } from "../strings/en";

/** How the signed-in person signs in: passkeys, the password, and every session. */
export function Account() {
  const words = en.account;
  const session = useQuery({ queryKey: ["session"], queryFn: readSession }).data;

  let shown = <p role="status">{words.loading}</p>;
  if (isData(session)) {
    const account = session.data.account;
    shown = (
      <>
        <p className="lede">{words.lede(account.name, account.email)}</p>
        <section className="max-w-160" aria-labelledby="passkeys-title">
          <h2 id="passkeys-title">{words.passkey.title}</h2>
          <p>{words.passkey.lede}</p>
          <Link to="/passkeys">{words.passkey.link}</Link>
        </section>
        <ChangePassword />
        <SignOutEverywhere />
      </>
    );
  } else if (session !== undefined) {
    // Only a session belongs to an account; a staff token belongs to none.
    const problem = session.status === 0 ? en.problems.unreachable : en.problems.noSession;
    shown = <Problem message={problem} />;
  }
  return (
    <>
      <h1>{words.title}</h1>
      {shown}
    </>
  );
}
