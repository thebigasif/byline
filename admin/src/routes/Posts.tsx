import { getRouteApi, Link } from "@tanstack/react-router";
import { useRows } from "../api/rows";
import { rowQuery } from "../posts/filters";
import { PostFilters } from "../posts/PostFilters";
import { PostResults } from "../posts/PostResults";
import { en } from "../strings/en";

const route = getRouteApi("/signed-in/posts");

/** Every post on the blog in any status, the latest change first, filtered by the address. */
export function Posts() {
  const search = route.useSearch();
  const rows = useRows(rowQuery(search));
  return (
    <>
      <div className="heading">
        <div>
          <h1>{en.posts.title}</h1>
          <p className="lede">{en.posts.lede}</p>
        </div>
        <Link to="/new" className="btn primary">
          {en.posts.write}
        </Link>
      </div>
      <dl className="terms">
        {en.posts.terms.map(({ term, meaning }) => (
          <div key={term}>
            <dt>{term}</dt>
            <dd>{meaning}</dd>
          </div>
        ))}
      </dl>
      <PostFilters search={search} />
      <PostResults rows={rows} search={search} />
    </>
  );
}
