import { problemMessage } from "../api/posts";
import { isPage, type Rows } from "../api/rows";
import { Problem } from "../components/Problem";
import { en } from "../strings/en";
import type { PostSearch } from "./filters";
import { Paging } from "./Paging";
import { PostTable } from "./PostTable";

type Props = { rows: Rows | undefined; search: PostSearch };

/** What a list call found: how many posts match and the page of them, or the problem. */
export function PostResults({ rows, search }: Props) {
  if (rows === undefined) {
    return <p role="status">{en.posts.loading}</p>;
  }
  if (!isPage(rows)) {
    return <Problem message={problemMessage(rows.problem, en.problems.unreachable)} />;
  }
  const total = rows.pagination.total;
  return (
    <>
      <p role="status">{total === 0 ? en.posts.none : en.posts.found(total)}</p>
      {rows.rows.length === 0 ? null : <PostTable rows={rows.rows} />}
      <Paging pagination={rows.pagination} search={search} />
    </>
  );
}
