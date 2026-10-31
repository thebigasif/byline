import { useQuery } from "@tanstack/react-query";
import { AddPasskey } from "../account/AddPasskey";
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
        <AddPasskey email={account.email} />
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
