import { type JSONContent, type MarkdownToken, Node } from "@tiptap/core";
import { en } from "../../strings/en";

// Two plan 4.6 blocks are standard markdown that Tiptap's starter kit does
// not hold: tables and footnotes. Each keeps its markdown exactly as written,
// so a post with one opens in the rich-text view and saves unchanged.

function source(token: MarkdownToken): string {
  return typeof token.raw === "string" ? token.raw.trimEnd() : "";
}

/** A node that shows a block of markdown as a card and writes it back as it was. */
function kept(name: string, tokenName: string, label: string) {
  return Node.create({
    name,
    group: "block",
    atom: true,
    selectable: true,
    draggable: true,
    addAttributes() {
      return {
        source: {
          default: "",
          parseHTML: (element: HTMLElement) => element.getAttribute("data-source") ?? "",
          renderHTML: (attrs: Record<string, string>) => ({ "data-source": attrs.source }),
        },
      };
    },
    parseHTML() {
      return [{ tag: `div[data-block="${name}"]` }];
    },
    renderHTML({ node, HTMLAttributes }) {
      const text = typeof node.attrs.source === "string" ? node.attrs.source : "";
      return [
        "div",
        { ...HTMLAttributes, "data-block": name, class: "block-card" },
        ["span", { class: "block-name" }, label],
        ["pre", { class: "block-source" }, text],
      ];
    },
    markdownTokenName: tokenName,
    parseMarkdown: (token: MarkdownToken): JSONContent => ({
      type: name,
      attrs: { source: source(token) },
    }),
    renderMarkdown: (node: JSONContent) =>
      typeof node.attrs?.source === "string" ? node.attrs.source : "",
  });
}

/** A table, `| a | b |`, kept as written. */
export const MarkdownTable = kept("markdownTable", "table", en.blocks.table);

/** A footnote's text, `[^1]: ...` and its indented lines, kept as written. */
export const FootnoteText = kept("footnoteText", "footnoteText", en.blocks.footnote).extend({
  markdownTokenizer: {
    name: "footnoteText",
    level: "block",
    start: (src: string) => src.search(/^\[\^[^\]\s]+\]:/m),
    tokenize: (src: string) => {
      const found = /^\[\^[^\]\s]+\]:[^\n]*(?:\n(?: {4}|\t)[^\n]*)*(?:\n|$)/.exec(src);
      return found === null ? undefined : { type: "footnoteText", raw: found[0] };
    },
  },
});

/** A footnote mark in the text, `[^1]`, kept as written. */
export const FootnoteMark = Node.create({
  name: "footnoteMark",
  group: "inline",
  inline: true,
  atom: true,
  addAttributes() {
    return {
      label: {
        default: "",
        parseHTML: (element: HTMLElement) => element.getAttribute("data-footnote") ?? "",
        renderHTML: (attrs: Record<string, string>) => ({ "data-footnote": attrs.label }),
      },
    };
  },
  parseHTML() {
    return [{ tag: "sup[data-footnote]" }];
  },
  renderHTML({ node, HTMLAttributes }) {
    return ["sup", HTMLAttributes, typeof node.attrs.label === "string" ? node.attrs.label : ""];
  },
  markdownTokenName: "footnoteMark",
  markdownTokenizer: {
    name: "footnoteMark",
    level: "inline",
    start: (src: string) => src.indexOf("[^"),
    tokenize: (src: string) => {
      const found = /^\[\^([^\]\s]+)\](?!:)/.exec(src);
      return found === null ? undefined : { type: "footnoteMark", raw: found[0], label: found[1] };
    },
  },
  parseMarkdown: (token: MarkdownToken): JSONContent => ({
    type: "footnoteMark",
    attrs: { label: typeof token.label === "string" ? token.label : "" },
  }),
  renderMarkdown: (node: JSONContent) =>
    typeof node.attrs?.label === "string" ? `[^${node.attrs.label}]` : "",
});
