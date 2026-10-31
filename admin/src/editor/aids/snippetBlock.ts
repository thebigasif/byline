import type { Editor, JSONContent } from "@tiptap/core";
import { nodeName } from "../blocks/directiveNode";
import { BLOCK_SPECS } from "../blocks/specs";
import { fileText } from "../richText";

/** The selected blocks in markdown, as a snippet file holds them; empty when none are. */
export function selectedMarkdown(editor: Editor): string {
  const { selection } = editor.state;
  if (selection.empty) {
    return "";
  }
  const content: JSONContent[] = selection.content().content.toJSON() ?? [];
  const markdown = editor.markdown?.serialize({ type: "doc", content }) ?? "";
  return fileText(markdown).trim();
}

/** Puts the snippet called `slug` where the selected blocks were. */
export function placeSnippet(editor: Editor, slug: string) {
  const spec = BLOCK_SPECS.find((candidate) => candidate.name === "snippet");
  if (spec === undefined) {
    return;
  }
  const block = { type: nodeName(spec), attrs: { slug } };
  editor.chain().focus().deleteSelection().insertContent(block).run();
}
