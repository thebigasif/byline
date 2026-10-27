import type { Miss } from "../api/redirects";
import { day } from "../strings/dates";
import { en } from "../strings/en";

type Props = { misses: Miss[]; onSend: (miss: Miss, to: string) => void };

/** The logged addresses, most visited first, each with the page Khalid would send it to. */
export function MissTable({ misses, onSend }: Props) {
  const words = en.redirects.misses;
  return (
    <div className="scroll">
      <table aria-labelledby="misses-title">
        <thead>
          <tr>
            <th scope="col">{words.path}</th>
            <th scope="col">{words.hits}</th>
            <th scope="col">{words.last}</th>
            <th scope="col">{words.nearest}</th>
            <th scope="col">
              <span className="muted">{words.action}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {misses.map((miss) => {
            const to = miss.nearest;
            return (
              <tr key={miss.path}>
                <td data-label={words.path}>
                  <code>{miss.path}</code>
                </td>
                <td data-label={words.hits}>{miss.hits}</td>
                <td data-label={words.last}>
                  <time dateTime={miss.last_seen}>{day(miss.last_seen)}</time>
                </td>
                <td data-label={words.nearest}>
                  {to === null ? (
                    <span className="muted">{words.noneNear}</span>
                  ) : (
                    <code>{to}</code>
                  )}
                </td>
                <td>
                  {to === null ? null : (
                    <button
                      type="button"
                      className="btn"
                      aria-label={words.send(miss.path)}
                      onClick={() => onSend(miss, to)}
                    >
                      {words.sendText}
                    </button>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
