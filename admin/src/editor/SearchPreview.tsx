import type { Snippet } from "../api/check";
import { en } from "../strings/en";
import { shownAddress } from "./snippet";

type Props = { snippet: Snippet };

/** The post as a Google result, from the title and description its page will carry. */
export function SearchPreview({ snippet }: Props) {
  const words = en.search;
  return (
    <details open>
      <summary>{words.title}</summary>
      <div>
        <p className="hint">{words.lede}</p>
        <section className="serp" aria-label={words.result}>
          <p className="url">{shownAddress(snippet.url)}</p>
          <p className="head">{snippet.title}</p>
          <p className="desc">{snippet.description === "" ? words.none : snippet.description}</p>
        </section>
        <p className="hint">{words.lengths(snippet.title.length, snippet.description.length)}</p>
      </div>
    </details>
  );
}
