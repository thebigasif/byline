import { useQuery } from "@tanstack/react-query";
import { type ReactNode, useEffect, useRef, useState } from "react";
import {
  type Found,
  type StockPicture,
  type StockSource,
  searchPictures,
} from "../../api/pictures";
import { useSettled } from "../../api/postSearch";
import { pictures as words } from "../../strings/pictures";

type Props = {
  source: StockSource;
  onPick: (picture: StockPicture) => void;
  onCancel: () => void;
};

/** What the search shows under the box: nothing yet, a wait, a failure, or the pictures. */
function results(found: Found | undefined, onPick: (picture: StockPicture) => void): ReactNode {
  if (found === undefined) {
    return (
      <p className="hint" role="status">
        {words.searching}
      </p>
    );
  }
  if ("failed" in found) {
    return <p role="alert">{found.failed}</p>;
  }
  if (found.pictures.length === 0) {
    return <p className="hint">{words.none}</p>;
  }
  return (
    <ul className="picture-grid" aria-label={words.found}>
      {found.pictures.map((picture) => (
        <li key={picture.id}>
          <button type="button" onClick={() => onPick(picture)}>
            <img
              src={picture.preview}
              alt=""
              width={picture.width}
              height={picture.height}
              loading="lazy"
            />
            <span>{words.by(picture.description, picture.credit)}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}

/**
 * A box to find a free photo on Unsplash or a GIF on GIPHY. It says where the
 * picture will load from and what goes under it before the writer picks one.
 */
export function PictureSearch({ source, onPick, onCancel }: Props) {
  const [typed, setTyped] = useState("");
  const settled = useSettled(typed.trim(), 300);
  const box = useRef<HTMLInputElement>(null);
  useEffect(() => {
    box.current?.focus();
  }, []);
  const found = useQuery({
    queryKey: ["pictures", source, settled],
    queryFn: () => searchPictures(source, settled, words.failed),
    enabled: settled !== "",
  }).data;
  const unsplash = source === "unsplash";
  return (
    <form
      className="focus wide block-form"
      aria-labelledby="picture-search-title"
      onSubmit={(event) => event.preventDefault()}
    >
      <h2 id="picture-search-title">{unsplash ? words.unsplash : words.gifs}</h2>
      <p className="hint">{unsplash ? words.unsplashHint : words.gifsHint}</p>
      <label htmlFor="picture-words">{words.words}</label>
      <input
        ref={box}
        id="picture-words"
        type="search"
        value={typed}
        onChange={(event) => setTyped(event.target.value)}
      />
      {settled === "" ? null : results(found, onPick)}
      <button type="button" className="btn" onClick={onCancel}>
        {words.cancel}
      </button>
    </form>
  );
}
