import { type JSONContent, type MarkdownToken, Node } from "@tiptap/core";
import { en } from "../../strings/en";
import { printOpening, readOpening, sameValues } from "./directive";
import { BLOCK_SPECS, type BlockSpec } from "./specs";

/** What a directive's markdown holds once it is read. */
type Read = {
  fence: string;
  opening: string;
  body: string;
  raw: string;
};

/**
 * Finds the directive `spec` at the start of `src`: a leaf's one line, or a
 * container's opening line, body and closing line of the same colons.
 */
export function readDirective(spec: BlockSpec, src: string): Read | null {
  const end = src.indexOf("\n");
  const first = end < 0 ? src : src.slice(0, end);
  const opening = readOpening(first);
  if (opening === null || opening.name !== spec.name) {
    return null;
  }
  const rest = first.slice(opening.fence.length).trimEnd();
  if (spec.leaf) {
    return opening.fence === "::"
      ? { fence: "::", opening: rest, body: "", raw: lineWithBreak(src, end) }
      : null;
  }
  if (opening.fence.length < 3 || end < 0) {
    return null;
  }
  const lines = src.slice(end + 1).split("\n");
  const close = lines.findIndex((line) => line.trimEnd() === opening.fence);
  if (close < 0) {
    return null;
  }
  // The closing line ends at `after`, where its line break is, if any.
  const after = end + 1 + lines.slice(0, close + 1).join("\n").length;
  return {
    fence: opening.fence,
    opening: rest,
    body: lines.slice(0, close).join("\n"),
    raw: src.slice(0, after + 1),
  };
}

/** `src` up to and with the line break at `end`, or all of it at -1. */
function lineWithBreak(src: string, end: number): string {
  return end < 0 ? src : src.slice(0, end + 1);
}

const labels: Record<string, string> = en.blocks;

/** Writes a block node back as its directive. */
export function printDirective(spec: BlockSpec, attrs: Record<string, string | null>): string {
  const written = readOpening(`${attrs.fence ?? spec.fence}${attrs.opening ?? ""}`);
  const opening =
    written !== null && sameValues(written.attributes, spec.attributes, attrs)
      ? (attrs.opening ?? spec.name)
      : printOpening(spec.name, spec.attributes, attrs);
  if (spec.leaf) {
    return `::${opening}`;
  }
  const fence = attrs.fence ?? spec.fence;
  const body = attrs.body ? `${attrs.body}\n` : "";
  return `${fence}${opening}\n${body}${fence}`;
}

/** The node name of a directive, kept apart from Tiptap's own nodes. */
export const nodeName = (spec: BlockSpec) => `${spec.name}Block`;

/** A Tiptap node for `spec` that reads and writes its directive unchanged. */
export function directiveNode(spec: BlockSpec) {
  const token = `${spec.name}Directive`;
  const own = ["fence", "opening", "body", ...spec.attributes];
  return Node.create({
    name: nodeName(spec),
    group: "block",
    atom: true,
    selectable: true,
    draggable: true,
    addAttributes() {
      return Object.fromEntries(
        own.map((key) => [
          key,
          {
            default: null,
            parseHTML: (element: HTMLElement) => element.getAttribute(`data-${key}`),
            renderHTML: (attrs: Record<string, string | null>) =>
              attrs[key] === null ? {} : { [`data-${key}`]: attrs[key] },
          },
        ]),
      );
    },
    parseHTML() {
      return [{ tag: `div[data-block="${spec.name}"]` }];
    },
    renderHTML({ node, HTMLAttributes }) {
      const detail = spec.shows === null ? null : node.attrs[spec.shows];
      return [
        "div",
        { ...HTMLAttributes, "data-block": spec.name, class: "block-card" },
        ["span", { class: "block-name" }, labels[spec.name] ?? spec.name],
        ["span", { class: "block-detail" }, typeof detail === "string" ? detail : ""],
      ];
    },
    markdownTokenName: token,
    markdownTokenizer: {
      name: token,
      level: "block",
      start: (src: string) => src.search(new RegExp(`^:{2,}${spec.name}\\b`, "m")),
      tokenize: (src: string) => {
        const read = readDirective(spec, src);
        return read === null ? undefined : { type: token, ...read };
      },
    },
    parseMarkdown: (found: MarkdownToken): JSONContent => {
      const fence = text(found.fence);
      const opening = text(found.opening);
      const attributes = readOpening(`${fence}${opening}`)?.attributes ?? [];
      const values = Object.fromEntries(
        spec.attributes.map((key) => [key, attributes.find(([k]) => k === key)?.[1] ?? null]),
      );
      return {
        type: nodeName(spec),
        attrs: { ...values, fence, opening, body: spec.leaf ? null : text(found.body) },
      };
    },
    renderMarkdown: (node: JSONContent) => printDirective(spec, node.attrs ?? {}),
  });
}

function text(value: unknown): string {
  return typeof value === "string" ? value : "";
}

/** One node per directive block. */
export const directiveNodes = BLOCK_SPECS.map(directiveNode);
