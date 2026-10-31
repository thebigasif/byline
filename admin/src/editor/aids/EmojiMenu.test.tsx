import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { writing } from "../../../test/editor";
import { EMOJI, findEmoji } from "./emoji";

describe("finding an emoji by its name", () => {
  it("puts the exact name first, then names that start with the words", () => {
    const names = findEmoji("heart").map(({ name }) => name);
    expect(names[0]).toBe("heart");
    expect(names[1]).toBe("heart_eyes");
    expect(names).toContain("orange_heart");
  });

  it("reads the words in any case", () => {
    expect(findEmoji("TADA")).toEqual([{ name: "tada", emoji: "🎉" }]);
  });

  it("shows at most eight", () => {
    expect(findEmoji("a")).toHaveLength(8);
  });

  it("finds nothing for words no name has", () => {
    expect(findEmoji("zzzz")).toEqual([]);
  });

  it("knows each name once, each with an emoji", () => {
    const names = EMOJI.map(({ name }) => name);
    expect(new Set(names).size).toBe(names.length);
    expect(EMOJI.length).toBeGreaterThan(200);
    for (const { name, emoji } of EMOJI) {
      expect(name).toMatch(/^[a-z0-9_+-]+$/);
      expect(emoji).not.toMatch(/[a-z]/);
    }
  });
});

describe("adding an emoji while writing", () => {
  it("puts the emoji itself in the text", () => {
    const { type, press, onChange } = writing();
    type("We shipped it :tad");
    expect(screen.getByRole("region", { name: "Add an emoji" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /:tada:/ })).toBeVisible();
    press("Enter");
    expect(onChange).toHaveBeenLastCalledWith("We shipped it 🎉\n");
    expect(screen.queryByRole("region", { name: "Add an emoji" })).toBeNull();
  });

  it("moves to the next emoji with the arrow key", () => {
    const { type, press, onChange } = writing();
    type(":heart");
    press("ArrowDown");
    press("Enter");
    expect(onChange).toHaveBeenLastCalledWith("😍\n");
  });

  it("leaves the colon and the words after Escape", () => {
    const { type, press, onChange } = writing();
    type("Ratio :sm");
    press("Escape");
    expect(screen.queryByRole("region", { name: "Add an emoji" })).toBeNull();
    expect(onChange).toHaveBeenLastCalledWith("Ratio :sm\n");
  });

  it("says so when no emoji has the name", () => {
    const { type } = writing();
    type(":qqq");
    expect(screen.getByText(/No emoji has that name/)).toBeInTheDocument();
  });
});
