import { useQuery } from "@tanstack/react-query";
import { problemMessage } from "../api/posts";
import { isData } from "../api/redirects";
import { readFollowUps } from "../api/searchConsole";
import { Problem } from "../components/Problem";
import { OpenSearches } from "../searchConsole/OpenSearches";
import { PostSearches } from "../searchConsole/PostSearches";
import { en } from "../strings/en";
import { searchConsoleWords } from "../strings/searchConsoleWords";

/** What Search Console says about the posts: the long searches no heading answers yet. */
export function SearchConsole() {
  const words = searchConsoleWords;
  // Google is asked once per visit; the answer does not change within minutes.
  const reply = useQuery({
    queryKey: ["follow-ups"],
    queryFn: readFollowUps,
    staleTime: 5 * 60 * 1000,
  }).data;

  let body = <p role="status">{words.loading}</p>;
  if (isData(reply)) {
    const found = reply.data;
    body = (
      <>
        <OpenSearches found={found} />
        {found.posts.length > 0 && (
          <>
            <h2>{words.posts.title}</h2>
            <dl className="terms">
              {words.posts.terms.map(({ term, meaning }) => (
                <div key={term}>
                  <dt>{term}</dt>
                  <dd>{meaning}</dd>
                </div>
              ))}
            </dl>
            {found.posts.map((post, index) => (
              <PostSearches key={post.url} post={post} id={`post-${index + 1}`} />
            ))}
          </>
        )}
        <p className="muted">{words.window}</p>
      </>
    );
  } else if (reply !== undefined) {
    body = <Problem message={problemMessage(reply.problem, en.problems.unreachable)} />;
  }
  return (
    <>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      {body}
    </>
  );
}
