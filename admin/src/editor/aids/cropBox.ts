import type { CropBox } from "./pictureTurn";

/** The smallest side of a box the writer draws, in thousandths: 5% (decision E13). */
export const MIN_SIDE = 50;

/** The box "Your own box" starts with: all but a tenth on each side. */
export const START: CropBox = { left: 100, top: 100, width: 800, height: 800 };

/** A point on the picture, in thousandths of its width and height. */
export type Point = { x: number; y: number };

/** What the writer is dragging: the whole box or its corner, from `from` on `box`. */
export type Drag = { part: "box" | "corner"; from: Point; box: CropBox };

/** One of the four fields that type a box in percent. */
export type Side = keyof CropBox;

/** Every field, in the order the form offers them. */
export const SIDES: readonly Side[] = ["left", "top", "width", "height"];

const clamp = (value: number, low: number, high: number) =>
  Math.min(Math.max(Math.round(value), low), high);

/** `box` moved by `x` and `y` thousandths, kept inside the picture. */
export function moved(box: CropBox, x: number, y: number): CropBox {
  return {
    ...box,
    left: clamp(box.left + x, 0, 1000 - box.width),
    top: clamp(box.top + y, 0, 1000 - box.height),
  };
}

/** `box` with its lower right corner moved by `x` and `y`, kept inside and at least 5%. */
export function resized(box: CropBox, x: number, y: number): CropBox {
  return {
    ...box,
    width: clamp(box.width + x, MIN_SIDE, 1000 - box.left),
    height: clamp(box.height + y, MIN_SIDE, 1000 - box.top),
  };
}

/** The box after the pointer went from where `drag` started to `to`. */
export function dragged(drag: Drag, to: Point): CropBox {
  const x = to.x - drag.from.x;
  const y = to.y - drag.from.y;
  return drag.part === "box" ? moved(drag.box, x, y) : resized(drag.box, x, y);
}

/** Where a pointer at `clientX`, `clientY` is on a picture drawn in `rect`. */
export function pointAt(clientX: number, clientY: number, rect: DOMRectReadOnly): Point | null {
  if (rect.width <= 0 || rect.height <= 0) {
    return null;
  }
  return {
    x: ((clientX - rect.left) * 1000) / rect.width,
    y: ((clientY - rect.top) * 1000) / rect.height,
  };
}

/** `side` of `box` in whole percent, as the fields show it. */
export function percentOf(box: CropBox, side: Side): number {
  return Math.round(box[side] / 10);
}

/** `box` with `side` typed as `percent`, kept inside the picture and at least 5%. */
export function withPercent(box: CropBox, side: Side, percent: number): CropBox {
  if (!Number.isFinite(percent)) {
    return box;
  }
  const value = percent * 10;
  switch (side) {
    case "left":
      return { ...box, left: clamp(value, 0, 1000 - box.width) };
    case "top":
      return { ...box, top: clamp(value, 0, 1000 - box.height) };
    case "width":
      return { ...box, width: clamp(value, MIN_SIDE, 1000 - box.left) };
    default:
      return { ...box, height: clamp(value, MIN_SIDE, 1000 - box.top) };
  }
}
