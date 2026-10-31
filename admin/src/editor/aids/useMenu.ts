import { type RefObject, useEffect, useState } from "react";

/** Handles a key the editor saw first; `true` when the open menu used it. */
export type KeyHandler = (event: KeyboardEvent) => boolean;

/** Lets every key through to the editor. */
export const noKeys: KeyHandler = () => false;

/** What a key does to an open menu. */
export type MenuKey = "up" | "down" | "pick" | "close" | null;

/** Reads a key as a menu move: arrows move, Enter and Tab pick, Escape closes. */
export function menuKey(event: KeyboardEvent): MenuKey {
  if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) {
    return null;
  }
  switch (event.key) {
    case "ArrowUp":
      return "up";
    case "ArrowDown":
      return "down";
    case "Enter":
    case "Tab":
      return "pick";
    case "Escape":
      return "close";
    default:
      return null;
  }
}

/**
 * Moves through `count` lines of an open menu with the keyboard, while the
 * cursor stays in the text. Gives back the line that is lit.
 */
export function useMenu(
  keys: RefObject<KeyHandler>,
  count: number,
  pick: (index: number) => void,
  close: () => void,
): number {
  const [lit, setLit] = useState(0);
  const active = count === 0 ? 0 : Math.min(lit, count - 1);
  useEffect(() => {
    keys.current = (event) => {
      const key = menuKey(event);
      if (key === "close") {
        close();
        return true;
      }
      if (count === 0 || key === null) {
        return false;
      }
      if (key === "pick") {
        pick(active);
      } else {
        setLit((active + (key === "down" ? 1 : count - 1)) % count);
      }
      return true;
    };
    return () => {
      keys.current = noKeys;
    };
  });
  return active;
}
