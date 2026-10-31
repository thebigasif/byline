import { useQuery } from "@tanstack/react-query";
import { type SearchStates, type StockSource, searchStates } from "../api/pictures";
import { searches as words } from "../strings/searches";

/** The searches, in the order Settings lists them. */
const SOURCES: readonly StockSource[] = ["unsplash", "gifs"];

/** The list, a wait, or what to say when the blog cannot tell. */
function shown(states: SearchStates | null | undefined) {
  if (states === undefined) {
    return <p role="status">{words.loading}</p>;
  }
  if (states === null) {
    return <p role="alert">{words.failed}</p>;
  }
  return (
    <ul className="search-states">
      {SOURCES.map((source) => {
        const state = states[source];
        return (
          <li key={source} className={state === "missing" ? "careful" : undefined}>
            <strong>{words.line(source, state)}</strong>
            <span className="hint">{words.detail(source, state)}</span>
          </li>
        );
      })}
    </ul>
  );
}

/**
 * Whether photo and GIF search work, for the Settings screen. It changes
 * nothing: the owner turns them on in `site.toml` and sets the keys on the
 * server, so a key never passes through the browser (decision E15).
 */
export function PictureSearches() {
  const states = useQuery({ queryKey: ["picture-search-states"], queryFn: searchStates }).data;
  return (
    <section aria-labelledby="picture-searches-title">
      <h2 id="picture-searches-title">{words.title}</h2>
      <p className="hint">{words.lede}</p>
      {shown(states)}
    </section>
  );
}
