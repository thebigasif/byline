import type { Visitors } from "../api/analytics";
import { count } from "../strings/dates";
import { en } from "../strings/en";

type Props = { pages: Visitors["pages"] };

/** The period's most read pages, each a link to the page on the blog. */
export function TopPages({ pages }: Props) {
  const words = en.analytics;
  return (
    <section aria-labelledby="pages-title">
      <h2 id="pages-title">{words.pages.title}</h2>
      {pages.length === 0 ? (
        <p>{words.pages.none}</p>
      ) : (
        <div className="scroll">
          <table aria-labelledby="pages-title">
            <thead>
              <tr>
                <th scope="col">{words.pages.page}</th>
                <th scope="col">{words.views}</th>
              </tr>
            </thead>
            <tbody>
              {pages.map((page) => (
                <tr key={page.path}>
                  <td data-label={words.pages.page}>
                    <a href={page.path}>
                      <code>{page.path}</code>
                    </a>
                  </td>
                  <td data-label={words.views}>{count(page.views)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
