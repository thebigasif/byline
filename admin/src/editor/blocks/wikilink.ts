import { type JSONContent, type MarkdownToken, Node } from "@tiptap/core";

// A wikilink, `[[slug]]` or `[[slug|words]]`, names a post, page or glossary
// term by its slug. `khalid-content` turns it into a link to that page's
// address at build, so the link follows the page when its address changes.
const WIKILINK = /^\[\[([^[\]|\n]+)(?:\|([^[\]\n]*))?\]\]/;

/** The markdown of a wikilink to `slug`, with `text` when the words differ. */
export function wikilinkMarkdown(slug: string, text: string | null): string {
  return text === null ? `[[${slug}]]` : `[[${slug}|${text}]]`;
}

/**
 * `words` as a wikilink's words, or `null` when they hold a bracket or a line
 * break that would end the wikilink early. A bare `[[slug]]` shows the page's
 * title at build, so nothing is lost.
 */
export function wikilinkWords(words: string): string | null {
  return words === "" || /[[\]\n]/.test(words) ? null : words;
}

function written(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

/** The words a wikilink shows: its own, or its slug. */
function shownWords(attrs: Record<string, unknown>): string {
  return written(attrs.text) ?? written(attrs.slug) ?? "";
}

/** A wikilink in the text: shown as its words, written back as typed. */
export const Wikilink = Node.create({
  name: "wikilink",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      slug: {
        default: "",
        parseHTML: (element: HTMLElement) => element.getAttribute("data-wikilink") ?? "",
        renderHTML: (attrs: Record<string, string>) => ({ "data-wikilink": attrs.slug }),
      },
      text: {
        default: null,
        parseHTML: (element: HTMLElement) => element.getAttribute("data-text"),
        renderHTML: (attrs: Record<string, string | null>) =>
          attrs.text === null ? {} : { "data-text": attrs.text },
      },
    };
  },
  parseHTML() {
    return [{ tag: "span[data-wikilink]" }];
  },
  renderHTML({ node, HTMLAttributes }) {
    return ["span", { ...HTMLAttributes, class: "wikilink" }, shownWords(node.attrs)];
  },
  renderText: ({ node }) => shownWords(node.attrs),
  markdownTokenName: "wikilink",
  markdownTokenizer: {
    name: "wikilink",
    level: "inline",
    start: (src: string) => src.indexOf("[["),
    tokenize: (src: string) => {
      const found = WIKILINK.exec(src);
      if (found === null) {
        return undefined;
      }
      return { type: "wikilink", raw: found[0], slug: found[1], text: found[2] ?? null };
    },
  },
  parseMarkdown: (token: MarkdownToken): JSONContent => ({
    type: "wikilink",
    attrs: { slug: written(token.slug) ?? "", text: written(token.text) },
  }),
  renderMarkdown: (node: JSONContent) =>
    wikilinkMarkdown(written(node.attrs?.slug) ?? "", written(node.attrs?.text)),
});
