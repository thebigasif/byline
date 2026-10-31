import { describe, expect, it } from "vitest";
import { BLOCK_SPECS } from "../blocks/specs";
import { MENU_LENGTH, matchingItems, SLASH_ITEMS } from "./slashItems";

const ids = (query: string) => matchingItems(query).map((item) => item.id);

describe("the slash menu's blocks", () => {
  it("offers every directive block", () => {
    for (const spec of BLOCK_SPECS) {
      expect(SLASH_ITEMS.some((item) => item.spec === spec)).toBe(true);
    }
  });

  it("names and explains every block", () => {
    for (const item of SLASH_ITEMS) {
      expect(item.name).not.toBe("");
      expect(item.hint).not.toBe("");
    }
  });

  it("shows the first few blocks before anything is typed", () => {
    expect(ids("")).toEqual(SLASH_ITEMS.slice(0, MENU_LENGTH).map((item) => item.id));
  });

  it("finds a block by its name or its directive", () => {
    expect(ids("call")[0]).toBe("callout");
    expect(ids("Toc")[0]).toBe("toc");
    expect(ids("table of")[0]).toBe("toc");
  });

  it("finds a block by a word of what it does", () => {
    expect(ids("steps")).toEqual(expect.arrayContaining(["howto", "numbers"]));
  });

  it("puts a name that starts with the words before a hint that holds them", () => {
    expect(ids("video")[0]).toBe("video");
  });

  it("finds nothing for words no block has", () => {
    expect(ids("zzz")).toEqual([]);
  });
});
