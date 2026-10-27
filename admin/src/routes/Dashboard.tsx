import { problemMessage } from "../api/posts";
import { isFailure, isPage, useRows } from "../api/rows";
import { Problem } from "../components/Problem";
import { StartPost } from "../components/StartPost";
import { BlockedFocus } from "../dashboard/BlockedFocus";
import { RecentChanges } from "../dashboard/RecentChanges";
import { en } from "../strings/en";

/** The first screen after signing in: the blog at a glance and the one thing to do next. */
export function Dashboard() {
  // A list counts its whole length, so one row is enough to learn a total.
  const blocked = useRows({ filter: "gate:refuses", limit: 5 });
  const recent = useRows({ order: "updated_at desc", limit: 5 });
  const live = useRows({ filter: "status:published", limit: 1 });
  const scheduled = useRows({ filter: "status:scheduled", limit: 1 });
  const drafts = useRows({ filter: "status:draft", limit: 1 });

  const failed = [blocked, recent, live, scheduled, drafts].find(isFailure);
  if (failed !== undefined) {
    return (
      <>
        <h1>{en.dashboard.title}</h1>
        <Problem message={problemMessage(failed.problem, en.problems.unreachable)} />
      </>
    );
  }
  if (!(isPage(blocked) && isPage(recent) && isPage(live) && isPage(scheduled) && isPage(drafts))) {
    return <p role="status">{en.dashboard.loading}</p>;
  }

  const stuck = blocked.pagination.total;
  const stats = [
    { count: live.pagination.total, label: en.dashboard.live },
    { count: scheduled.pagination.total, label: en.dashboard.scheduled },
    { count: drafts.pagination.total, label: en.dashboard.drafts },
    { count: stuck, label: en.dashboard.blocked },
  ];
  return (
    <>
      <h1>{en.dashboard.title}</h1>
      <p className="lede">{stuck === 0 ? en.dashboard.calm : en.dashboard.blockedLede(stuck)}</p>
      {stuck === 0 ? <StartPost /> : <BlockedFocus page={blocked} />}
      <h2>{en.dashboard.glance}</h2>
      <ul className="stats">
        {stats.map(({ count, label }) => (
          <li key={label(2)} className="card">
            <strong>{count}</strong>
            {label(count)}
          </li>
        ))}
      </ul>
      <h2>{en.dashboard.recent}</h2>
      <RecentChanges rows={recent.rows} />
    </>
  );
}
