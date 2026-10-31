import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { problemMessage } from "../api/posts";
import { listChanges } from "../api/records";
import { isData } from "../api/redirects";
import { AuditTable } from "../audit/AuditTable";
import { Problem } from "../components/Problem";
import { en } from "../strings/en";

/** Who changed what on the blog and when, newest first, a page at a time. */
export function AuditLog() {
  const words = en.audit;
  // Each page starts below the last change of the page before it.
  const [before, setBefore] = useState<number | undefined>(undefined);
  const page = useQuery({
    queryKey: ["audit", before],
    queryFn: () => listChanges(before),
  }).data;

  let list = <p role="status">{words.loading}</p>;
  if (page !== undefined && !isData(page)) {
    list = <Problem message={problemMessage(page.problem, en.problems.unreachable)} />;
  } else if (isData(page) && page.data.changes.length === 0) {
    list = <p role="status">{words.none}</p>;
  } else if (isData(page)) {
    const next = page.data.next;
    list = (
      <>
        <AuditTable changes={page.data.changes} />
        <div className="flex flex-wrap items-center gap-3">
          {next === null || next === undefined ? null : (
            <button type="button" className="btn" onClick={() => setBefore(next)}>
              {words.older}
            </button>
          )}
          {before === undefined ? null : (
            <button type="button" className="btn" onClick={() => setBefore(undefined)}>
              {words.newest}
            </button>
          )}
        </div>
      </>
    );
  }
  return (
    <>
      <h1 id="audit-title">{words.title}</h1>
      <p className="lede">{words.lede}</p>
      <dl className="terms">
        <div>
          <dt>{words.who}</dt>
          <dd>{words.whoMeans}</dd>
        </div>
      </dl>
      {list}
    </>
  );
}
