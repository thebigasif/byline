import { useQuery } from "@tanstack/react-query";
import { Link, Navigate, useLocation } from "@tanstack/react-router";
import { problemMessage } from "../api/posts";
import { isData } from "../api/redirects";
import { redeemLink } from "../api/sites";
import { Problem } from "../components/Problem";
import { en } from "../strings/en";

/** Signs in with a link someone was sent, then opens the Sites page. */
export function LinkSignIn() {
  const words = en.sites.link;
  // The token sits after the #, so it never reaches a server log.
  const hash = useLocation({ select: (location) => location.hash });
  const token = new URLSearchParams(hash.replace(/^#/, "")).get("token") ?? "";
  const redeemed = useQuery({
    queryKey: ["sign-in-link", token],
    queryFn: () => redeemLink(token),
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
  }).data;

  let shown = <p role="status">{words.checking}</p>;
  if (isData(redeemed)) {
    // A person's session reaches only the Sites page until the admin signs people in.
    shown = <Navigate to="/sites" />;
  } else if (redeemed !== undefined) {
    // The API's 401 speaks of passwords; a link that failed was used or old.
    const message =
      redeemed.status === 401
        ? en.sites.problems.badLink
        : problemMessage(redeemed.problem, en.sites.problems.badLink);
    shown = (
      <>
        <Problem message={message} />
        <Link to="/sites">{words.back}</Link>
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
