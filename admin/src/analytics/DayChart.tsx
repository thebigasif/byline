import type { Visitors } from "../api/analytics";
import { calendarDay, count } from "../strings/dates";
import { en } from "../strings/en";

type Props = { perDay: Visitors["per_day"] };

/** The bars leave a tenth of the chart's height above the busiest day. */
const TALLEST = 90;

/** The page views of each day as bars, said in words too, with every day in a table below. */
export function DayChart({ perDay }: Props) {
  const words = en.analytics.chart;
  const most = Math.max(0, ...perDay.map((row) => row.views));
  const busiest = perDay.find((row) => row.views === most);
  if (most === 0 || busiest === undefined) {
    return (
      <section aria-labelledby="days-title">
        <h2 id="days-title">{words.title}</h2>
        <p>{words.none}</p>
      </section>
    );
  }
  const least = Math.min(...perDay.map((row) => row.views));
  const summary = words.summary(count(least), count(most), calendarDay(busiest.day));
  return (
    <section aria-labelledby="days-title">
      <h2 id="days-title">{words.title}</h2>
      {/* The bars only draw what the summary and the table say, so readers skip them. */}
      <svg
        className="chart"
        viewBox={`0 0 ${perDay.length * 10} 100`}
        preserveAspectRatio="none"
        aria-hidden="true"
      >
        {perDay.map((row, at) => {
          const height = Math.round((row.views / most) * TALLEST);
          return <rect key={row.day} x={at * 10 + 1} y={100 - height} width={8} height={height} />;
        })}
      </svg>
      <p className="muted">{summary}</p>
      <details>
        <summary>{words.table}</summary>
        <div className="scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">{words.day}</th>
                <th scope="col">{en.analytics.views}</th>
              </tr>
            </thead>
            <tbody>
              {[...perDay].reverse().map((row) => (
                <tr key={row.day}>
                  <td data-label={words.day}>{calendarDay(row.day)}</td>
                  <td data-label={en.analytics.views}>{count(row.views)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
    </section>
  );
}
