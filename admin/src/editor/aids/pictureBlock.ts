import type { Editor } from "@tiptap/core";
import type { StockPicture } from "../../api/pictures";
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

/** Adds `picture` as a figure at `at`, where the writer typed the slash item. */
export function placePicture(editor: Editor, at: number, picture: StockPicture) {
  const figure = BLOCK_SPECS.find((spec) => spec.name === "figure");
  if (figure === undefined) {
    return;
  }
  const block = { type: nodeName(figure), attrs: figureFor(picture) };
  editor.chain().focus().insertContentAt(at, block).run();
}
