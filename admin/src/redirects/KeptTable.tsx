import type { StoredRedirect } from "../api/redirects";
import { en } from "../strings/en";

type Props = { redirects: StoredRedirect[]; onRemove: (redirect: StoredRedirect) => void };

/** The kept redirects: where each leads, what a visitor gets, who made it, and Remove. */
export function KeptTable({ redirects, onRemove }: Props) {
  const words = en.redirects.kept;
  return (
    <div className="scroll">
      <table aria-labelledby="kept-title">
        <thead>
          <tr>
            <th scope="col">{words.from}</th>
            <th scope="col">{words.to}</th>
            <th scope="col">{words.kind}</th>
            <th scope="col">{words.origin}</th>
            <th scope="col">
              <span className="muted">{words.action}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {redirects.map((redirect) => (
            <tr key={redirect.id}>
              <td data-label={words.from}>
                <code>{redirect.from}</code>
              </td>
              <td data-label={words.to}>
                {redirect.to === null ? (
                  <span className="muted">{words.nothing}</span>
                ) : (
                  <code>{redirect.to}</code>
                )}
              </td>
              <td data-label={words.kind}>
                {en.redirects.kind(redirect.status, redirect.match === "regex")}
              </td>
              <td data-label={words.origin}>
                <span className="pill quiet">{en.redirects.origins[redirect.origin]}</span>
              </td>
              <td>
                <button
                  type="button"
                  className="btn"
                  aria-label={words.remove(redirect.from)}
                  onClick={() => onRemove(redirect)}
                >
                  {words.removeText}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
