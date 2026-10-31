import type { Editor } from "@tiptap/core";
import type { RefObject } from "react";
import { usePostSearch } from "../../api/postSearch";
import { isFailure, isPage, type PostRow, type Rows } from "../../api/rows";
import { en } from "../../strings/en";
import { AidMenu } from "./AidMenu";
import { closeMenu, type Trigger } from "./trigger";
import { type KeyHandler, useMenu } from "./useMenu";

type Props = {
  editor: Editor;
  trigger: Trigger;
  keys: RefObject<KeyHandler>;
};

/**
 * Puts a wikilink to `post` where the writer typed `@words` or `[[words`. A
 * mention shows the post's title; `[[` writes the bare `[[slug]]`, which the
 * build shows as the title too.
 */
export function linkPost(editor: Editor, trigger: Trigger, post: PostRow) {
  const text = trigger.kind === "mention" ? post.title : null;
  const range = { from: trigger.from, to: trigger.to };
  const nodes = [
    { type: "wikilink", attrs: { slug: post.slug, text } },
    { type: "text", text: " " },
  ];
  editor.chain().focus().insertContentAt(range, nodes).run();
}

function emptyText(found: Rows | undefined): string {
  if (found === undefined) {
    return en.aids.searching;
  }
  return isFailure(found) ? en.aids.searchFailed : en.aids.noPost;
}

/** The posts that match what the writer typed after `@` or `[[`. */
export function PostMenu({ editor, trigger, keys }: Props) {
  const found = usePostSearch(trigger.query);
  const posts = isPage(found) ? found.rows : [];
  const pick = (index: number) => {
    const post = posts[index];
    if (post !== undefined) {
      linkPost(editor, trigger, post);
    }
  };
  const active = useMenu(keys, posts.length, pick, () => closeMenu(editor));
  const lines = posts.map((post) => ({ id: post.id, name: post.title, hint: post.slug }));
  return (
    <AidMenu
      label={trigger.kind === "mention" ? en.aids.mention : en.aids.wikilink}
      lines={lines}
      active={active}
      empty={emptyText(found)}
      onPick={pick}
    />
  );
}
