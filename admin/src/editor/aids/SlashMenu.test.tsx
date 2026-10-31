import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { writing } from "../../../test/editor";

const menu = () => screen.queryByRole("region", { name: "Add a block" });

describe("the slash menu", () => {
  it("opens on an empty line and narrows as the writer types", () => {
    const { type } = writing();
    type("/");
    expect(menu()).toBeInTheDocument();
    type("quo");
    expect(screen.getAllByRole("button").map((line) => line.textContent)).toEqual([
      "QuoteWords from someone else. In markdown, start the line with >",
    ]);
  });

  it("turns the line into the block picked with Enter", () => {
    const { editor, type, press, onChange } = writing();
    type("/quote");
    press("Enter");
    expect(editor.getJSON().content?.[0]?.type).toBe("blockquote");
    expect(menu()).not.toBeInTheDocument();
    expect(onChange).toHaveBeenLastCalledWith(expect.not.stringContaining("/quote"));
  });

  it("adds a directive block", () => {
    const { editor, type, press, onChange } = writing();
    type("/toc");
    press("Enter");
    expect(editor.getJSON().content?.[0]?.type).toBe("tocBlock");
    expect(onChange).toHaveBeenLastCalledWith(expect.stringMatching(/^::toc\n/));
  });

  it("moves with the arrow keys", () => {
    const { type, press } = writing();
    type("/");
    press("ArrowDown");
    expect(screen.getByRole("button", { current: true })).toHaveTextContent("Smaller heading");
    press("ArrowUp");
    press("ArrowUp");
    expect(screen.getByRole("button", { current: true })).toHaveTextContent("Audio");
  });

  it("adds the block the writer clicks", () => {
    const { editor, type } = writing();
    type("/");
    fireEvent.click(screen.getByRole("button", { name: /Dividing line/ }));
    const types = editor.getJSON().content?.map((node) => node.type);
    expect(types).toContain("horizontalRule");
    expect(editor.getText()).not.toContain("/");
  });

  it("closes with Escape and keeps what was typed", () => {
    const { editor, type, press } = writing();
    type("/call");
    press("Escape");
    expect(menu()).not.toBeInTheDocument();
    expect(editor.getText()).toBe("/call");
  });

  it("says so when no block matches", () => {
    const { type } = writing();
    type("/zzz");
    expect(menu()).toHaveTextContent("No block has that name.");
  });

  it("stays shut for a slash inside a sentence", () => {
    const { type } = writing();
    type("Half is 1/2");
    expect(menu()).not.toBeInTheDocument();
  });
});
