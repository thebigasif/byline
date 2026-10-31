import type { Editor } from "@tiptap/core";
import type { EditorState } from "@tiptap/pm/state";
import type { PostRow } from "../../api/rows";
import { wikilinkWords } from "../blocks/wikilink";

/** The words a link search was opened on, and where they sit; empty at a bare cursor. */
export type Chosen = { from: number; to: number; words: string };

/** The selected words, if they sit in one paragraph or heading, else the cursor alone. */
export function chosenIn(state: EditorState): Chosen {
  const { $from, $to, from, to } = state.selection;
  if (!$from.sameParent($to) || !$from.parent.isTextblock) {
    return { from: to, to, words: "" };
  }
  return { from, to, words: state.doc.textBetween(from, to) };
}

/** Whether `event` is Ctrl+K, or Cmd+K on a Mac, the key that opens link search. */
export function isLinkKey(event: KeyboardEvent): boolean {
  const command = event.ctrlKey || event.metaKey;
  return command && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "k";
}

/** Whether the writer typed a web address rather than words to find a post by. */
export function isAddress(words: string): boolean {
  return /^(?:https?:\/\/\S+|\/\S*)$/.test(words.trim());
}

/** Links the chosen words to `post` with a wikilink, or puts `[[slug]]` at the cursor. */
export function linkToPost(editor: Editor, chosen: Chosen, post: PostRow) {
  const text = wikilinkWords(chosen.words);
  const range = { from: chosen.from, to: chosen.to };
  const node = { type: "wikilink", attrs: { slug: post.slug, text } };
  editor.chain().focus().insertContentAt(range, node).run();
}

/** Links the chosen words to a web address, or puts the address itself at the cursor. */
export function linkToAddress(editor: Editor, chosen: Chosen, address: string) {
  const href = address.trim();
  if (chosen.words !== "") {
    const range = { from: chosen.from, to: chosen.to };
    editor.chain().focus().setTextSelection(range).setLink({ href }).run();
    return;
  }
  const text = { type: "text", text: href, marks: [{ type: "link", attrs: { href } }] };
  editor.chain().focus().insertContentAt(chosen.from, text).run();
}
