import { useInfiniteQuery } from "@tanstack/react-query";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { type Listed, listPictures } from "../../api/library";
import type { MediaItem } from "../../api/pictures";
import { useSettled } from "../../api/postSearch";
import { library as words } from "../../strings/library";

type Props = {
  onPick: (picture: MediaItem) => void;
  onCancel: () => void;
};

/** What a picture's button says: its alt text, or that it has none yet and its name. */
export function pictureName(picture: MediaItem): string {
  const alt = picture.alt.trim();
  return alt === "" ? words.noAlt(picture.name) : alt;
}

/** The pictures found so far, a wait, a failure, or an empty library. */
function shown(pages: Listed[] | undefined, searched: boolean, onPick: Props["onPick"]): ReactNode {
  if (pages === undefined) {
    return (
      <p className="hint" role="status">
        {words.loading}
      </p>
    );
  }
  const failed = pages.find((page) => "failed" in page);
  if (failed !== undefined && "failed" in failed) {
    return <p role="alert">{failed.failed}</p>;
  }
  const pictures = pages.flatMap((page) => ("pictures" in page ? page.pictures : []));
  if (pictures.length === 0) {
    return <p className="hint">{searched ? words.none : words.empty}</p>;
  }
  return (
    <ul className="picture-grid" aria-label={words.found}>
      {pictures.map((picture) => (
        <li key={picture.id}>
          <button type="button" onClick={() => onPick(picture)}>
            <img
              src={picture.url}
              alt=""
              width={picture.width ?? undefined}
              height={picture.height ?? undefined}
              loading="lazy"
            />
            <span>{pictureName(picture)}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}

/**
 * A list of the pictures in the media library, newest first, to add one to
 * the post. Typing narrows it by name or alt text (decision E12).
 */
export function LibraryPicker({ onPick, onCancel }: Props) {
  const [typed, setTyped] = useState("");
  const settled = useSettled(typed.trim(), 300);
  const box = useRef<HTMLInputElement>(null);
  useEffect(() => {
    box.current?.focus();
  }, []);
  const listed = useInfiniteQuery({
    queryKey: ["library-pictures", settled],
    queryFn: ({ pageParam }) => listPictures(settled, pageParam, words.failed),
    initialPageParam: 1,
    getNextPageParam: (last: Listed, all: Listed[]) =>
      "more" in last && last.more ? all.length + 1 : undefined,
  });
  return (
    <form
      className="focus wide block-form"
      aria-labelledby="library-title"
      onSubmit={(event) => event.preventDefault()}
    >
      <h2 id="library-title">{words.title}</h2>
      <p className="hint">{words.hint}</p>
      <label htmlFor="library-words">{words.words}</label>
      <input
        ref={box}
        id="library-words"
        type="search"
        value={typed}
        onChange={(event) => setTyped(event.target.value)}
      />
      {shown(listed.data?.pages, settled !== "", onPick)}
      <div className="flex flex-wrap gap-3">
        {listed.hasNextPage ? (
          <button
            type="button"
            className="btn"
            disabled={listed.isFetchingNextPage}
            onClick={() => void listed.fetchNextPage()}
          >
            {words.more}
          </button>
        ) : null}
        <button type="button" className="btn" onClick={onCancel}>
          {words.cancel}
        </button>
      </div>
    </form>
  );
}
