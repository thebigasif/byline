import { Link } from "@tanstack/react-router";
import type { PostRow } from "../api/rows";
import { day } from "../strings/dates";
import { en } from "../strings/en";

type Props = { rows: PostRow[] };

/** The posts changed last: what each is now, who changed it and when. */
export function RecentChanges({ rows }: Props) {
  if (rows.length === 0) {
    return <p>{en.dashboard.noPosts}</p>;
  }
  return (
    <div className="scroll">
      <table>
        <thead>
          <tr>
            <th scope="col">{en.dashboard.post}</th>
            <th scope="col">{en.dashboard.stands}</th>
            <th scope="col">{en.dashboard.who}</th>
            <th scope="col">{en.dashboard.when}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.id}>
              <td>
                <Link to="/posts/$id" params={{ id: row.id }}>
                  {row.title}
                </Link>
              </td>
              <td data-label={en.dashboard.stands}>{en.status[row.status]}</td>
              <td data-label={en.dashboard.who}>{row.updated_by ?? en.dashboard.nobody}</td>
              <td data-label={en.dashboard.when}>
                <time dateTime={row.updated_at}>{day(row.updated_at)}</time>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
