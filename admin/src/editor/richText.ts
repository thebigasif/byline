import { Editor } from "@tiptap/core";
import { Markdown } from "@tiptap/markdown";
import StarterKit from "@tiptap/starter-kit";
import { directiveNodes } from "./blocks/directiveNode";
import { FootnoteMark, FootnoteText, MarkdownTable } from "./blocks/markdownBlocks";
import { BLOCK_SPECS, INNER_BLOCKS } from "./blocks/specs";

/** The blocks the rich-text view knows, each with its markdown form. */
export const extensions = [
  StarterKit,
  Markdown,
  ...directiveNodes,
  MarkdownTable,
  FootnoteText,
  FootnoteMark,
];

/** The markdown the rich-text view writes for `markdown` after reading it. */
export function throughRichText(markdown: string): string {
  const editor = new Editor({ extensions, content: markdown, contentType: "markdown" });
  const out = editor.getMarkdown();
  editor.destroy();
  return out;
}

/**
 * Whether the rich-text view gives back exactly this markdown. A post with
 * a block it does not know yet, such as a wikilink or a directive with a
 * name Khalid does not have, opens as markdown instead, so nothing is ever
 * rewritten by surprise.
 */
export function fitsRichText(markdown: string): boolean {
  if (/\[\[/.test(markdown) || unknownDirective(markdown)) {
    return false;
  }
  return throughRichText(markdown).trimEnd() === markdown.trimEnd();
}

const KNOWN = new Set([...BLOCK_SPECS.map((spec) => spec.name), ...INNER_BLOCKS]);

// A directive line whose name is not a block would read as plain text.
const DIRECTIVE = /^\s*:{2,}([A-Za-z][\w-]*)/gm;

function unknownDirective(markdown: string): boolean {
  const names = [...markdown.matchAll(DIRECTIVE)].map((found) => found[1] ?? "");
  return names.some((name) => !KNOWN.has(name));
}

/** The file's text from the rich-text view, ending in one line break. */
export function fileText(richMarkdown: string): string {
  const text = richMarkdown.trimEnd();
  return text === "" ? "" : `${text}\n`;
}
