import { Extension } from "@tiptap/core";
import { type EditorState, Plugin, PluginKey } from "@tiptap/pm/state";

/** What the writer typed to open a menu: `/` for blocks, `@` or `[[` for a post. */
export type TriggerKind = "slash" | "mention" | "wikilink";

/** An open menu: its kind, the words typed after the trigger, and where they sit. */
export type Trigger = { kind: TriggerKind; query: string; from: number; to: number };

/** The trigger at the end of `before`, and how many characters it covers. */
export type Found = { kind: TriggerKind; query: string; length: number };

// `/` opens the block menu only on an otherwise empty line, as in the mockup,
// so a fraction such as 1/2 or a web address never opens it.
const SLASH = /^\/([\p{L}\p{N}-]{0,30})$/u;
// `@` after a space or at the line's start, so an email address never opens it.
const MENTION = /(?:^|\s)@([\p{L}\p{N}-]{0,40})$/u;
const WIKILINK = /\[\[([^[\]|\n]{0,60})$/;

/** Finds the trigger the writer is typing at the end of `before`, if any. */
export function findTrigger(before: string): Found | null {
  const slash = SLASH.exec(before);
  if (slash !== null) {
    return { kind: "slash", query: slash[1] ?? "", length: slash[0].length };
  }
  const wikilink = WIKILINK.exec(before);
  if (wikilink !== null) {
    return { kind: "wikilink", query: wikilink[1] ?? "", length: wikilink[0].length };
  }
  const mention = MENTION.exec(before);
  if (mention !== null) {
    const query = mention[1] ?? "";
    return { kind: "mention", query, length: query.length + 1 };
  }
  return null;
}

/** The trigger the cursor sits after in `state`, outside code, if any. */
export function triggerIn(state: EditorState): Trigger | null {
  const { selection } = state;
  const { $from } = selection;
  if (!selection.empty || !$from.parent.isTextblock || $from.parent.type.spec.code === true) {
    return null;
  }
  // Inline nodes count as one character that matches nothing.
  const before = $from.parent.textBetween(0, $from.parentOffset, undefined, "￼");
  const found = findTrigger(before);
  if (found === null) {
    return null;
  }
  return { kind: found.kind, query: found.query, from: $from.pos - found.length, to: $from.pos };
}

type AidsState = { trigger: Trigger | null; closedAt: number | null };

/** Where the writing aids keep the open menu. */
export const aidsKey = new PluginKey<AidsState>("writingAids");

/** The menu open in `state`, unless the writer closed it with Escape. */
export function openTrigger(state: EditorState): Trigger | null {
  const aids = aidsKey.getState(state);
  if (aids === undefined || aids.trigger === null || aids.trigger.from === aids.closedAt) {
    return null;
  }
  return aids.trigger;
}

/** A transaction meta that closes the open menu until a new trigger is typed. */
export const CLOSE = "close";

/** Watches the text for `/`, `@` and `[[`, so the editor can open the right menu. */
export const WritingAids = Extension.create({
  name: "writingAids",
  addProseMirrorPlugins() {
    return [
      new Plugin<AidsState>({
        key: aidsKey,
        state: {
          init: (_, state) => ({ trigger: triggerIn(state), closedAt: null }),
          apply: (tr, old, _, state) => {
            const trigger = triggerIn(state);
            if (tr.getMeta(aidsKey) === CLOSE) {
              return { trigger, closedAt: trigger?.from ?? null };
            }
            const kept = trigger !== null && trigger.from === old.closedAt;
            return { trigger, closedAt: kept ? old.closedAt : null };
          },
        },
      }),
    ];
  },
});
