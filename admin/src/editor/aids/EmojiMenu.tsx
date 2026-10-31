import type { Editor } from "@tiptap/core";
import type { RefObject } from "react";
import { emoji as words } from "../../strings/emoji";
import { AidMenu } from "./AidMenu";
import { type Emoji, findEmoji } from "./emoji";
import { closeMenu, type Trigger } from "./trigger";
import { type KeyHandler, useMenu } from "./useMenu";

type Props = {
  editor: Editor;
  trigger: Trigger;
  keys: RefObject<KeyHandler>;
};

/** Puts `chosen` itself where the writer typed `:name`, so the post file holds the emoji. */
export function placeEmoji(editor: Editor, trigger: Trigger, chosen: Emoji) {
  const range = { from: trigger.from, to: trigger.to };
  editor.chain().focus().insertContentAt(range, chosen.emoji).run();
}

/** The emoji whose names match what the writer typed after `:`. */
export function EmojiMenu({ editor, trigger, keys }: Props) {
  const found = findEmoji(trigger.query);
  const pick = (index: number) => {
    const chosen = found[index];
    if (chosen !== undefined) {
      placeEmoji(editor, trigger, chosen);
    }
  };
  const active = useMenu(keys, found.length, pick, () => closeMenu(editor));
  const lines = found.map((one) => ({ id: one.name, name: one.emoji, hint: `:${one.name}:` }));
  return (
    <AidMenu label={words.title} lines={lines} active={active} empty={words.none} onPick={pick} />
  );
}
