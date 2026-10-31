/** How far a picture turns clockwise, in degrees. */
export type Degrees = 0 | 90 | 180 | 270;

/** The shapes the editor crops to, around the picture's focal point. */
export type Shape = "taken" | "square" | "wide" | "standing";

/** Every shape, in the order the form offers them. */
export const SHAPES: readonly Shape[] = ["taken", "square", "wide", "standing"];

/** Width to height of each shape but the picture's own. */
const RATIOS: Record<Exclude<Shape, "taken">, [number, number]> = {
  square: [1, 1],
  wide: [16, 9],
  standing: [3, 4],
};

/** A box in thousandths of the turned picture, as the Admin API takes it. */
export type CropBox = { left: number; top: number; width: number; height: number };

/** What the editor knows of a library picture: its size and focal point. */
export type PictureFacts = { width: number; height: number; x: number; y: number };

/** The turn after a quarter turn to the right, or to the left. */
export function turned(degrees: Degrees, right: boolean): Degrees {
  const next = (degrees + (right ? 90 : 270)) % 360;
  return next === 90 || next === 180 || next === 270 ? next : 0;
}

/** The library key in a picture's address, such as `.../media/library/<key>/<name>`. */
export function libraryKey(src: string): string | null {
  const found = /\/media\/library\/([0-9a-f]{16})\/[^/?#]+$/.exec(src.trim());
  return found?.[1] ?? null;
}

/**
 * The SVG transform that draws a `width` by `height` picture turned by
 * `degrees`, with its turned corner at the origin.
 */
export function turnTransform(degrees: Degrees, width: number, height: number): string {
  switch (degrees) {
    case 90:
      return `translate(${height} 0) rotate(90)`;
    case 180:
      return `translate(${width} ${height}) rotate(180)`;
    case 270:
      return `translate(0 ${width}) rotate(270)`;
    default:
      return "";
  }
}

/** `facts` after turning by `degrees`: the sides swap on a quarter, the focal point moves. */
export function afterTurn(facts: PictureFacts, degrees: Degrees): PictureFacts {
  const { width, height, x, y } = facts;
  switch (degrees) {
    case 90:
      return { width: height, height: width, x: 1000 - y, y: x };
    case 180:
      return { width, height, x: 1000 - x, y: 1000 - y };
    case 270:
      return { width: height, height: width, x: y, y: 1000 - x };
    default:
      return facts;
  }
}

/** Where a box `length` long starts on a side `side` long, centred on `at` thousandths. */
function place(side: number, length: number, at: number): number {
  const start = Math.round((side * at) / 1000 - length / 2);
  return Math.min(Math.max(start, 0), side - length);
}

/**
 * The largest box of `shape` in the picture after `degrees`, centred on its
 * focal point as far as the edges allow, in thousandths; `null` keeps all.
 */
export function cropFor(shape: Shape, degrees: Degrees, facts: PictureFacts): CropBox | null {
  if (shape === "taken") {
    return null;
  }
  const [across, down] = RATIOS[shape];
  const { width, height, x, y } = afterTurn(facts, degrees);
  const wide = width * down >= height * across;
  const boxWidth = wide ? Math.max(1, Math.round((height * across) / down)) : width;
  const boxHeight = wide ? height : Math.max(1, Math.round((width * down) / across));
  const share = (part: number, side: number) => Math.floor((part * 1000) / side);
  const left = share(place(width, boxWidth, x), width);
  const top = share(place(height, boxHeight, y), height);
  return {
    left,
    top,
    width: Math.min(Math.max(share(boxWidth, width), 1), 1000 - left),
    height: Math.min(Math.max(share(boxHeight, height), 1), 1000 - top),
  };
}
