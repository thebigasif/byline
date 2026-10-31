import type { AiVisitors } from "../api/analytics";
import { count } from "../strings/dates";
import { en } from "../strings/en";

type Props = { report: AiVisitors };

/** The readers each AI assistant sent, against the period before, and where they landed most. */
export function AiTools({ report }: Props) {
  const words = en.analytics;
  const before = words.before(report.days);
  return (
    <section aria-labelledby="tools-title">
      <h2 id="tools-title">{words.tools.title}</h2>
      <p className="muted">{words.tools.lede}</p>
      {report.tools.length === 0 ? (
        <p>{words.tools.none}</p>
      ) : (
        <div className="scroll">
          <table aria-labelledby="tools-title">
            <thead>
              <tr>
                <th scope="col">{words.tools.tool}</th>
                <th scope="col">{words.views}</th>
                <th scope="col">{before}</th>
                <th scope="col">{words.tools.top}</th>
              </tr>
            </thead>
            <tbody>
              {report.tools.map((row) => {
                const top = row.pages[0];
                return (
                  <tr key={row.tool}>
                    <th scope="row">{row.name}</th>
                    <td data-label={words.views}>{count(row.views)}</td>
                    <td data-label={before}>{count(row.before)}</td>
                    <td data-label={words.tools.top}>
                      {top === undefined ? (
                        <span className="muted">{words.tools.noPage}</span>
                      ) : (
                        <code>{top.path}</code>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
