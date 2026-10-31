import { describe, expect, it } from "vitest";
import { dragged, moved, percentOf, pointAt, resized, START, withPercent } from "./cropBox";
import { afterTurn, turnTransform } from "./pictureTurn";

const BOX = { left: 200, top: 150, width: 500, height: 600 };

/** A rectangle on screen, as the browser measures the picture. */
const rect = (left: number, top: number, width: number, height: number) =>
  ({ left, top, width, height }) as DOMRectReadOnly;

describe("a crop box the writer drags", () => {
  it("moves inside the picture and stops at its edges", () => {
    expect(moved(BOX, 50, -50)).toEqual({ ...BOX, left: 250, top: 100 });
    expect(moved(BOX, 900, 900)).toEqual({ ...BOX, left: 500, top: 400 });
    expect(moved(BOX, -900, -900)).toEqual({ ...BOX, left: 0, top: 0 });
  });

  it("grows from its corner up to the edges, and keeps at least 5% a side", () => {
    expect(resized(BOX, 100, 100)).toEqual({ ...BOX, width: 600, height: 700 });
    expect(resized(BOX, 900, 900)).toEqual({ ...BOX, width: 800, height: 850 });
    expect(resized(BOX, -900, -900)).toEqual({ ...BOX, width: 50, height: 50 });
  });

  it("follows the pointer from where the drag started", () => {
    const from = { x: 400, y: 400 };
    expect(dragged({ part: "box", from, box: BOX }, { x: 450, y: 420 })).toEqual(
      moved(BOX, 50, 20),
    );
    expect(dragged({ part: "corner", from, box: BOX }, { x: 300, y: 400 })).toEqual(
      resized(BOX, -100, 0),
    );
  });

  it("reads a pointer on the drawn picture in thousandths", () => {
    expect(pointAt(150, 75, rect(100, 50, 200, 100))).toEqual({ x: 250, y: 250 });
    expect(pointAt(150, 75, rect(100, 50, 0, 0))).toBeNull();
  });

  it("starts with all but a tenth on each side", () => {
    expect(START).toEqual({ left: 100, top: 100, width: 800, height: 800 });
  });
});

describe("the turned picture under the box", () => {
  it("swaps the sides on a quarter turn", () => {
    const facts = { width: 400, height: 200, x: 500, y: 500 };
    expect(afterTurn(facts, 90)).toMatchObject({ width: 200, height: 400 });
    expect(afterTurn(facts, 180)).toMatchObject({ width: 400, height: 200 });
  });

  it("draws the picture turned with its new corner at the origin", () => {
    expect(turnTransform(0, 400, 200)).toBe("");
    expect(turnTransform(90, 400, 200)).toBe("translate(200 0) rotate(90)");
    expect(turnTransform(180, 400, 200)).toBe("translate(400 200) rotate(180)");
    expect(turnTransform(270, 400, 200)).toBe("translate(0 400) rotate(270)");
  });
});

describe("a crop box typed in percent", () => {
  it("shows each side in whole percent", () => {
    expect(percentOf({ ...BOX, left: 205 }, "left")).toBe(21);
    expect(percentOf(BOX, "height")).toBe(60);
  });

  it("keeps a typed side inside the picture and at least 5%", () => {
    expect(withPercent(BOX, "left", 30)).toEqual({ ...BOX, left: 300 });
    expect(withPercent(BOX, "top", 90)).toEqual({ ...BOX, top: 400 });
    expect(withPercent(BOX, "width", 100)).toEqual({ ...BOX, width: 800 });
    expect(withPercent(BOX, "height", 1)).toEqual({ ...BOX, height: 50 });
  });

  it("ignores a field that holds no number", () => {
    expect(withPercent(BOX, "width", Number.NaN)).toBe(BOX);
  });
});
