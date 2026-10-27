import { Editor } from "@tiptap/core";
import { Markdown } from "@tiptap/markdown";
import StarterKit from "@tiptap/starter-kit";

/** The blocks the rich-text view knows, each with its markdown form. */
export const extensions = [StarterKit, Markdown];

/** The markdown the rich-text view writes for `markdown` after reading it. */
export function throughRichText(markdown: string): string {
  const editor = new Editor({ extensions, content: markdown, contentType: "markdown" });
  const out = editor.getMarkdown();
  editor.destroy();
  return out;
}

/**
 * Whether the rich-text view gives back exactly this markdown. A post with
 * a block it does not know yet, such as a directive or a wikilink, opens
 * as markdown instead, so nothing is ever rewritten by surprise.
 */
export function fitsRichText(markdown: string): boolean {
  if (BEYOND_RICH_TEXT.some((pattern) => pattern.test(markdown))) {
    return false;
  }
  return throughRichText(markdown).trimEnd() === markdown.trimEnd();
}

// Block and leaf directives, and wikilinks, which would read as plain text
// and could come back escaped after an edit.
const BEYOND_RICH_TEXT = [/^\s*::/m, /\[\[/];

/** The file's text from the rich-text view, ending in one line break. */
export function fileText(richMarkdown: string): string {
  const text = richMarkdown.trimEnd();
  return text === "" ? "" : `${text}\n`;
}
