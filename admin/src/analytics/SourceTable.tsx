import type { Visitors } from "../api/analytics";
import { count } from "../strings/dates";
import { en } from "../strings/en";

type Props = { counts: Visitors };

const NAMES: Record<string, string> = en.analytics.sources.names;

/** Where the period's readers came from, each source against the period before. */
export function SourceTable({ counts }: Props) {
  const words = en.analytics;
  const before = words.before(counts.days);
  return (
    <section aria-labelledby="sources-title">
      <h2 id="sources-title">{words.sources.title}</h2>
      <p className="muted">{words.sources.lede(count(counts.arrivals))}</p>
      <div className="scroll">
        <table aria-labelledby="sources-title">
          <thead>
            <tr>
              <th scope="col">{words.sources.from}</th>
              <th scope="col">{words.views}</th>
              <th scope="col">{before}</th>
            </tr>
          </thead>
          <tbody>
            {counts.sources.map((row) => (
              <tr key={row.source}>
                <th scope="row">{NAMES[row.source] ?? row.source}</th>
                <td data-label={words.views}>{count(row.views)}</td>
                <td data-label={before}>{count(row.before)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
