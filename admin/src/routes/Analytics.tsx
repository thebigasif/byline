import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { DayChart } from "../analytics/DayChart";
import { PeriodTotal } from "../analytics/PeriodTotal";
import { SourceTable } from "../analytics/SourceTable";
import { type Days, readVisitors } from "../api/analytics";
import { problemMessage } from "../api/posts";
import { type Failure, isData } from "../api/redirects";
import { Problem } from "../components/Problem";
import { en } from "../strings/en";

/** The problem a report came back with, if it came back with one. */
function failure(reply: { data: unknown } | Failure | undefined): Failure | undefined {
  return reply !== undefined && !("data" in reply) ? reply : undefined;
}

/** How the blog is read: page views, where readers came from, and what AI assistants do with it. */
export function Analytics() {
  const [days, setDays] = useState<Days>(28);
  // The last answer stays on screen while another period is read.
  const kept = { placeholderData: keepPreviousData };
  const visitors = useQuery({
    queryKey: ["visitors", days],
    queryFn: () => readVisitors(days),
    ...kept,
  }).data;

  const failed = failure(visitors);
  let body = <p role="status">{en.analytics.loading}</p>;
  if (failed !== undefined) {
    body = <Problem message={problemMessage(failed.problem, en.problems.unreachable)} />;
  } else if (isData(visitors)) {
    body = (
      <>
        <PeriodTotal counts={visitors.data} days={days} onDays={setDays} />
        <DayChart perDay={visitors.data.per_day} />
        <SourceTable counts={visitors.data} />
      </>
    );
  }
  return (
    <>
      <h1>{en.analytics.title}</h1>
      <p className="lede">{en.analytics.lede}</p>
      {body}
    </>
  );
}
