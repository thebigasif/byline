import type { Uncited } from "../api/analytics";
import { count, day } from "../strings/dates";
import { en } from "../strings/en";

type Props = { report: Uncited };

/** The pages AI bots read but no AI assistant sent a reader to, with what to do about them. */
export function UncitedPages({ report }: Props) {
  const words = en.analytics.uncited;
  return (
    <section aria-labelledby="uncited-title">
      <h2 id="uncited-title">{words.title}</h2>
      <p className="muted">{words.lede}</p>
      {report.pages.length === 0 ? (
        <p>{words.none}</p>
      ) : (
        <div className="scroll">
          <table aria-labelledby="uncited-title">
            <thead>
              <tr>
                <th scope="col">{en.analytics.pages.page}</th>
                <th scope="col">{words.reads}</th>
                <th scope="col">{words.bots}</th>
                <th scope="col">{words.last}</th>
              </tr>
            </thead>
            <tbody>
              {report.pages.map((page) => (
                <tr key={page.path}>
                  <td data-label={en.analytics.pages.page}>
                    <code>{page.path}</code>
                  </td>
                  <td data-label={words.reads}>{count(page.reads)}</td>
                  <td data-label={words.bots}>{page.bots.join(", ")}</td>
                  <td data-label={words.last}>
                    <time dateTime={page.last_read}>{day(page.last_read)}</time>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="muted">{words.after}</p>
    </section>
  );
}
