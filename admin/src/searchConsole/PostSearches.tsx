import type { FollowUpPost } from "../api/searchConsole";
import { count } from "../strings/dates";
import { en } from "../strings/en";
import { searchConsoleWords } from "../strings/searchConsoleWords";

type Props = { post: FollowUpPost; id: string };

const POSITION = new Intl.NumberFormat(en.locale, {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
});

/** One post's long searches, each with the heading that answers it or "No heading yet". */
export function PostSearches({ post, id }: Props) {
  const words = searchConsoleWords.posts;
  return (
    <section aria-labelledby={id}>
      <h3 id={id}>
        <a href={post.url}>{post.title}</a>
      </h3>
      <div className="scroll">
        <table aria-labelledby={id}>
          <thead>
            <tr>
              <th scope="col">{words.search}</th>
              <th scope="col">{words.shown}</th>
              <th scope="col">{words.clicks}</th>
              <th scope="col">{words.position}</th>
              <th scope="col">{words.answeredBy}</th>
            </tr>
          </thead>
          <tbody>
            {post.searches.map((search) => (
              <tr key={search.query}>
                <td data-label={words.search}>{search.query}</td>
                <td data-label={words.shown}>{count(Math.round(search.impressions))}</td>
                <td data-label={words.clicks}>{count(Math.round(search.clicks))}</td>
                <td data-label={words.position}>{POSITION.format(search.position)}</td>
                <td data-label={words.answeredBy}>
                  {search.answered_by == null ? (
                    <span className="pill warn">{words.noHeading}</span>
                  ) : (
                    words.heading(search.answered_by)
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
