import { useNavigate } from "@tanstack/react-router";
import { en } from "../strings/en";
import { MediaResults } from "./MediaResults";
import type { MediaSearch } from "./search";

type Props = { search: MediaSearch };

/** The library, newest first, with a switch to only the images still missing alt text. */
export function Library({ search }: Props) {
  const words = en.media.library;
  const navigate = useNavigate();
  const noAlt = search.show === "no-alt";
  const show = (only: boolean) => {
    void navigate({ to: "/media", search: only ? { show: "no-alt" } : {} });
  };
  return (
    <section aria-labelledby="library-title">
      <h2 id="library-title">{words.title}</h2>
      <p className="muted">{words.lede}</p>
      <fieldset className="switch" aria-label={words.show}>
        <button type="button" aria-pressed={!noAlt} onClick={() => show(false)}>
          {words.every}
        </button>
        <button type="button" aria-pressed={noAlt} onClick={() => show(true)}>
          {words.noAlt}
        </button>
      </fieldset>
      <MediaResults search={search} />
    </section>
  );
}
