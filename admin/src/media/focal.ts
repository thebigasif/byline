/** A focal point in whole percent of the width and height, from the top left. */
export type Percent = { x: number; y: number };

/** A whole percent from 0 to 100, from what a field or a click gave. */
export function percent(value: number): number {
  return Number.isFinite(value) ? Math.min(100, Math.max(0, Math.round(value))) : 0;
}
