import type { Editor } from "@tiptap/core";
import { act, fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { writing } from "../../../test/editor";
import { axeViolations } from "../../../test/render";

/** Types `text` one key at a time, the way the browser hands keys to the editor. */
function keyIn(editor: Editor, text: string) {
  for (const key of text) {
    const { from, to } = editor.state.selection;
    const typed = () => editor.state.tr.insertText(key, from, to);
    const used = editor.view.someProp("handleTextInput", (handle) =>
      handle(editor.view, from, to, key, typed),
    );
    if (used !== true) {
      editor.view.dispatch(typed());
    }
  }
}

// Each markdown shortcut, and the markdown the post's file gets for it.
const SHORTCUTS: [string, string][] = [
  ["## Plan it", "## Plan it"],
  ["### Write it", "### Write it"],
  ["- Feed the starter", "- Feed the starter"],
  ["1. Pick the subject", "1. Pick the subject"],
  ["> Write short posts first", "> Write short posts first"],
  ["Pick **your customers** first", "Pick **your customers** first"],
  ["Write the *short* posts", "Write the *short* posts"],
  ["Run `khalid lint` now", "Run `khalid lint` now"],
];

describe("markdown shortcuts", () => {
  for (const [typed, markdown] of SHORTCUTS) {
    it(`turn "${typed}" into formatting`, () => {
      const { editor, onChange } = writing();
      act(() => keyIn(editor, typed));
      expect(onChange).toHaveBeenLastCalledWith(`${markdown}\n`);
    });
  }

  it("make a dividing line from three dashes", () => {
    const { editor } = writing();
    act(() => keyIn(editor, "---"));
    expect(editor.getJSON().content?.map((node) => node.type)).toContain("horizontalRule");
  });

  it("make a code block from three backticks and a space", () => {
    const { editor } = writing();
    act(() => keyIn(editor, "``` "));
    expect(editor.getJSON().content?.[0]?.type).toBe("codeBlock");
  });
});

describe("the list of shortcuts", () => {
  it("opens with Ctrl+/ and closes with its button", async () => {
    const { press } = writing();
    press("/", { ctrlKey: true });
    const list = screen.getByRole("region", { name: "Writing shortcuts" });
    expect(list).toHaveTextContent("Link the selected words to a post or a web address");
    expect(screen.getByRole("button", { name: "Close the list" })).toHaveFocus();
    expect(await axeViolations()).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: "Close the list" }));
    expect(screen.queryByRole("region", { name: "Writing shortcuts" })).not.toBeInTheDocument();
  });
});
