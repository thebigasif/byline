import type { Editor } from "@tiptap/core";
import type { RefObject } from "react";
import { en } from "../../strings/en";
import { AidMenu } from "./AidMenu";
import { matchingItems, type SlashItem } from "./slashItems";
import { aidsKey, CLOSE, type Trigger } from "./trigger";
import { type KeyHandler, useMenu } from "./useMenu";

type Props = {
  editor: Editor;
  trigger: Trigger;
  keys: RefObject<KeyHandler>;
};

/** Closes the open menu and leaves what the writer typed in the text. */
export function closeMenu(editor: Editor) {
  editor.view.dispatch(editor.state.tr.setMeta(aidsKey, CLOSE));
}

/** Turns the line holding `/words` into the block `item`. */
export function addBlock(editor: Editor, trigger: Trigger, item: SlashItem) {
  const chain = editor.chain().focus().deleteRange({ from: trigger.from, to: trigger.to });
  item.add(chain).run();
}

/** The blocks that match what the writer typed after `/`. */
export function SlashMenu({ editor, trigger, keys }: Props) {
  const items = matchingItems(trigger.query);
  const pick = (index: number) => {
    const item = items[index];
    if (item !== undefined) {
      addBlock(editor, trigger, item);
    }
  };
  const active = useMenu(keys, items.length, pick, () => closeMenu(editor));
  return (
    <AidMenu
      label={en.aids.slash}
      lines={items}
      active={active}
      empty={en.aids.noBlock}
      onPick={pick}
    />
  );
}
