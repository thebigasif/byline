import { Link } from "@tanstack/react-router";
import type { RowPage } from "../api/rows";
import { en } from "../strings/en";

type Props = { page: RowPage };

/** The posts the linter stops from going live, as the dashboard's one focus point. */
export function BlockedFocus({ page }: Props) {
  const total = page.pagination.total;
  return (
    <section className="focus" aria-labelledby="focus-title">
      <h2 id="focus-title" className="mt-0">
        {en.dashboard.blockedTitle(total)}
      </h2>
      <p>{en.dashboard.blockedWhy}</p>
      <ul className="posts">
        {page.rows.map((row) => (
          <li key={row.id}>
            <Link to="/posts/$id" params={{ id: row.id }}>
              {row.title}
            </Link>
            {row.author === null || row.author === undefined ? null : (
              <span className="muted">{` · ${row.author}`}</span>
            )}
          </li>
        ))}
      </ul>
      <p>{en.dashboard.blockedFix}</p>
      <Link to="/posts" search={{ checks: "blocked" }} className="btn primary">
        {en.dashboard.showBlocked}
      </Link>
    </section>
  );
}
