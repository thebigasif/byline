import { useQuery } from "@tanstack/react-query";
import { Link, Navigate, useLocation } from "@tanstack/react-router";
import { redeemLink } from "../api/account";
import { isData } from "../api/call";
import { problemMessage } from "../api/posts";
import { saveCsrf } from "../auth/session";
import { Problem } from "../components/Problem";
import { en } from "../strings/en";

/** Signs in with a link someone was sent, then opens the admin. */
export function LinkSignIn() {
  const words = en.signIn.link;
  // The token sits after the #, so it never reaches a server log.
  const hash = useLocation({ select: (location) => location.hash });
  const token = new URLSearchParams(hash.replace(/^#/, "")).get("token") ?? "";
  const redeemed = useQuery({
    queryKey: ["sign-in-link", token],
    queryFn: async () => {
      const reply = await redeemLink(token);
      if (isData(reply)) {
        saveCsrf(reply.data.csrf);
      }
      return reply;
    },
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
  }).data;

  let shown = <p role="status">{words.checking}</p>;
  if (isData(redeemed)) {
    shown = <Navigate to="/" />;
  } else if (redeemed !== undefined) {
    shown = (
      <>
        <Problem message={problemMessage(redeemed.problem, en.problems.badLink)} />
        <Link to="/sign-in">{words.back}</Link>
      </>
    );
  }
  return (
    <main id="main" className="page">
      <h1>{words.title}</h1>
      {shown}
    </main>
  );
}
