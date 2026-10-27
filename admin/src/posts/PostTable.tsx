import { Link } from "@tanstack/react-router";
import type { PostRow } from "../api/rows";
import { day } from "../strings/dates";
import { en } from "../strings/en";
import { CheckPills } from "./CheckPills";
import { stageOf } from "./stages";

type Props = { rows: PostRow[] };

/** One page of posts: title, author, checks, stage, what search engines are told, last change. */
export function PostTable({ rows }: Props) {
  return (
    <div className="scroll">
      <table className="wide">
        <thead>
          <tr>
            <th scope="col">{en.posts.titleColumn}</th>
            <th scope="col">{en.posts.author}</th>
            <th scope="col">{en.posts.checks}</th>
            <th scope="col">{en.posts.review}</th>
            <th scope="col">{en.posts.index}</th>
            <th scope="col">{en.posts.changed}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => {
            const stage = stageOf(row);
            return (
              <tr key={row.id}>
                <td>
                  <Link to="/posts/$id" params={{ id: row.id }}>
                    {row.title}
                  </Link>
                  <br />
                  <span className="muted">{`/${row.slug}/`}</span>
                </td>
                <td data-label={en.posts.author}>{row.author ?? en.posts.noAuthor}</td>
                <td data-label={en.posts.checks}>
                  <CheckPills lint={row.lint} />
                </td>
                <td data-label={en.posts.review}>
                  <span className={stage === "live" ? "pill ok" : "pill quiet"}>
                    {en.stage[stage]}
                  </span>
                </td>
                <td data-label={en.posts.index}>
                  <span className={row.index_status === "indexable" ? "pill ok" : "pill quiet"}>
                    {en.index[row.index_status]}
                  </span>
                </td>
                <td data-label={en.posts.changed}>
                  <time dateTime={row.updated_at}>{day(row.updated_at)}</time>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
