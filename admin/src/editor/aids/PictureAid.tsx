import type { Editor } from "@tiptap/core";
import { photoUsed } from "../../api/pictures";
import { LibraryPicker } from "./LibraryPicker";
import { PictureSearch } from "./PictureSearch";
import { figureFor, libraryFigure, placeFigure } from "./pictureBlock";
import type { PictureSource } from "./slashItems";

/** A picture being found, and where it goes in the post. */
export type Finding = { source: PictureSource; at: number };

type Props = {
  editor: Editor;
  finding: Finding;
  /** Called once the picture is in the post. */
  onPlaced: () => void;
  onCancel: () => void;
};

/** The box that finds a picture for the post, from the library or a search. */
export function PictureAid({ editor, finding, onPlaced, onCancel }: Props) {
  const { source, at } = finding;
  if (source === "library") {
    return (
      <LibraryPicker
        onPick={(picture) => {
          placeFigure(editor, at, libraryFigure(picture));
          onPlaced();
        }}
        onCancel={onCancel}
      />
    );
  }
  return (
    <PictureSearch
      source={source}
      onPick={(picture) => {
        placeFigure(editor, at, figureFor(picture));
        if (picture.source === "unsplash") {
          void photoUsed(picture.id);
        }
        onPlaced();
      }}
      onCancel={onCancel}
    />
  );
}
