import { Link } from "@tanstack/react-router";
import type { SerpdadDraft } from "../api/serpdad";
import { CheckPills } from "../posts/CheckPills";
import { day } from "../strings/dates";
import { en } from "../strings/en";

type Props = { drafts: SerpdadDraft[] };

/** The drafts SerpDad sent, newest first: each opens in the editor, with its checks. */
export function DraftsTable({ drafts }: Props) {
  const words = en.serpdad.drafts;
  let list = <p role="status">{words.none}</p>;
  if (drafts.length > 0) {
    list = (
      <div className="scroll">
        <table aria-labelledby="serpdad-drafts-title">
          <thead>
            <tr>
              <th scope="col">{words.draft}</th>
              <th scope="col">{words.checks}</th>
              <th scope="col">{words.arrived}</th>
            </tr>
          </thead>
          <tbody>
            {drafts.map((draft) => (
              <tr key={`${draft.id} ${draft.received_at}`}>
                <td>
                  <Link to="/posts/$id" params={{ id: draft.id }}>
                    {draft.title}
                  </Link>
                </td>
                <td data-label={words.checks}>
                  {draft.lint === null || draft.lint === undefined ? (
                    <span className="pill quiet">{words.unchecked}</span>
                  ) : (
                    <CheckPills lint={draft.lint} />
                  )}
                </td>
                <td data-label={words.arrived}>
                  <time dateTime={draft.received_at}>{day(draft.received_at)}</time>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }
  return (
    <section aria-labelledby="serpdad-drafts-title">
      <h2 id="serpdad-drafts-title">{words.title}</h2>
      <p className="muted">{words.lede}</p>
      {list}
    </section>
  );
}
