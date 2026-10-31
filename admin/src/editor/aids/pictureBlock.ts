import type { Editor } from "@tiptap/core";
import type { MediaItem, StockPicture } from "../../api/pictures";
import { pictures as words } from "../../strings/pictures";
import { nodeName } from "../blocks/directiveNode";
import { BLOCK_SPECS } from "../blocks/specs";

/** The figure block's attributes for `picture`: its address, alt text and credit. */
export type FigureAttrs = { src: string; alt: string; body: string };

/** `text` safe inside a markdown link's words. */
function linkWords(text: string): string {
  return text.replace(/[[\]\\]/g, (found) => `\\${found}`);
}

/**
 * The figure for `picture`, credited as its source asks: "Photo by <name> on
 * Unsplash" with both linked, or "Via GIPHY" linked to the GIF's page.
 */
export function figureFor(picture: StockPicture): FigureAttrs {
  const alt = picture.description.trim() || words.photoBy(picture.credit);
  if (picture.source !== "unsplash") {
    return { src: picture.address, alt, body: `Via [GIPHY](${picture.page})` };
  }
  const credit = `[${linkWords(picture.credit)}](${picture.credit_link})`;
  const body = `Photo by ${credit} on [Unsplash](${picture.page})`;
  return { src: picture.address, alt, body };
}

/**
 * The figure for library item `picture`: its address, with the alt text and
 * caption saved with it in the library (decision E12).
 */
export function libraryFigure(picture: MediaItem): FigureAttrs {
  return { src: picture.url, alt: picture.alt.trim(), body: picture.caption.trim() };
}

/** Adds a figure with `attrs` at `at`, where the writer typed the slash item. */
export function placeFigure(editor: Editor, at: number, attrs: FigureAttrs) {
  const figure = BLOCK_SPECS.find((spec) => spec.name === "figure");
  if (figure === undefined) {
    return;
  }
  const block = { type: nodeName(figure), attrs };
  editor.chain().focus().insertContentAt(at, block).run();
}
