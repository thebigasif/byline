import type { Editor } from "@tiptap/core";
import type { RefObject } from "react";
import { usePictureSources } from "../../api/pictures";
import { en } from "../../strings/en";
import { AidMenu } from "./AidMenu";
import { type Details, newDetails } from "./blockForm";
import { matchingItems, type PictureSource, type SlashItem, TABLE_ITEM } from "./slashItems";
import { closeMenu, type Trigger } from "./trigger";
import { type KeyHandler, useMenu } from "./useMenu";

type Props = {
  editor: Editor;
  trigger: Trigger;
  keys: RefObject<KeyHandler>;
  /** Opens the details form of a block that needs some before it is added. */
  onDetails: (details: Details) => void;
  /** Opens an empty table as a grid, to add where `/table` was once filled in. */
  onTable: (at: number) => void;
  /** Opens photo or GIF search or the library, to add the picked picture where `/words` was. */
  onPictures: (source: PictureSource, at: number) => void;
};

/** Turns the line holding `/words` into the block `item`. */
export function addBlock(editor: Editor, trigger: Trigger, item: SlashItem) {
  const chain = editor.chain().focus().deleteRange({ from: trigger.from, to: trigger.to });
  item.add(chain).run();
}

/** The blocks that match what the writer typed after `/`. */
export function SlashMenu({ editor, trigger, keys, onDetails, onTable, onPictures }: Props) {
  const items = matchingItems(trigger.query, usePictureSources());
  const pick = (index: number) => {
    const item = items[index];
    const details = item?.spec ? newDetails(item.spec, trigger.from) : null;
    // The form adds the block where `/words` was, once its details are in.
    const clear = () => editor.chain().deleteRange({ from: trigger.from, to: trigger.to }).run();
    if (details !== null) {
      clear();
      onDetails(details);
    } else if (item?.id === TABLE_ITEM) {
      clear();
      onTable(trigger.from);
    } else if (item?.source !== undefined) {
      clear();
      onPictures(item.source, trigger.from);
    } else if (item !== undefined) {
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
