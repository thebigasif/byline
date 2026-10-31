import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { problemMessage } from "../api/posts";
import { isData } from "../api/redirects";
import { listSites, type Site } from "../api/sites";
import { readCsrf } from "../auth/session";
import { Problem } from "../components/Problem";
import { AddSite } from "../sites/AddSite";
import { AskForLink } from "../sites/AskForLink";
import { SiteTable } from "../sites/SiteTable";
import { en } from "../strings/en";

const SITES = ["sites"];

/**
 * The super-admin panel: every site this server runs, and adding one.
 * It sits outside the signed-in shell, since it needs a person's session
 * where every other screen needs a key.
 */
export function Sites() {
  const queries = useQueryClient();
  const csrf = readCsrf();
  const listed = useQuery({
    queryKey: SITES,
    queryFn: listSites,
    enabled: csrf !== null,
  }).data;

  const added = (site: Site) => {
    queries.setQueryData(SITES, isData(listed) ? { data: [...listed.data, site] } : listed);
  };

  const words = en.sites;
  let shown: ReactNode = <p role="status">{words.loading}</p>;
  const ended = listed !== undefined && "status" in listed && listed.status === 401;
  if (csrf === null || ended) {
    // No session in this tab, or it ended: the page needs the person to sign in.
    shown = <AskForLink />;
  } else if (isData(listed)) {
    shown = (
      <>
        <AddSite onAdded={added} />
        <SiteTable sites={listed.data} />
      </>
    );
  } else if (listed !== undefined) {
    shown = <Problem message={problemMessage(listed.problem, en.problems.unreachable)} />;
  }
  return (
    <main id="main" className="page">
      <p>
        <Link to="/">{words.openAdmin}</Link>
      </p>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      {shown}
    </main>
  );
}
