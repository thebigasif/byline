import { Link } from "@tanstack/react-router";
import { useRows } from "../api/rows";
import { PostResults } from "../posts/PostResults";
import { en } from "../strings/en";

/** How many rows one page of the posts screen holds. */
const PAGE_SIZE = 20;

/** Every post on the blog in any status, the latest change first. */
export function Posts() {
  const rows = useRows({ order: "updated_at desc", limit: PAGE_SIZE });
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
      <PostResults rows={rows} />
    </>
  );
}
