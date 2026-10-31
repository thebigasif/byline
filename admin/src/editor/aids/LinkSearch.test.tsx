import { act, fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { writing } from "../../../test/editor";
import { serve } from "../../../test/render";
import { row, rowsPage } from "../../../test/rows";
import { isAddress } from "./linkTo";

const POST = "Read why links matter.";

/** The writer selects `words` in POST and presses Ctrl+K. */
function linking(from: number, to: number) {
  serve(() => rowsPage([row()]));
  const view = writing(POST);
  act(() => void view.editor.commands.setTextSelection({ from, to }));
  view.press("k", { ctrlKey: true });
  return view;
}

const find = (words: string) =>
  fireEvent.change(screen.getByLabelText("Find a post, or paste a web address"), {
    target: { value: words },
  });

describe("telling a web address from words", () => {
  it("knows a full or a site address", () => {
    expect(isAddress("https://web.dev/articles/vitals")).toBe(true);
    expect(isAddress(" /pricing/ ")).toBe(true);
  });

  it("reads anything else as words to find a post by", () => {
    expect(isAddress("internal links")).toBe(false);
    expect(isAddress("web.dev")).toBe(false);
  });
});

describe("link search", () => {
  it("links the selected words to a post with a wikilink", async () => {
    const { onChange } = linking(6, 22);
    expect(screen.getByRole("form", { name: 'Link "why links matter"' })).toBeInTheDocument();
    expect(screen.getByLabelText("Find a post, or paste a web address")).toHaveFocus();
    find("internal");
    fireEvent.click(await screen.findByRole("button", { name: /Why internal links matter/ }));
    expect(onChange).toHaveBeenLastCalledWith(
      "Read [[why-internal-links-matter|why links matter]].\n",
    );
    expect(screen.queryByRole("form")).not.toBeInTheDocument();
  });

  it("links the selected words to a web address", () => {
    const { onChange } = linking(1, 5);
    find("https://web.dev/articles/vitals");
    expect(screen.getByText(/Press Enter to link to https:\/\/web\.dev/)).toBeVisible();
    fireEvent.submit(screen.getByRole("form"));
    expect(onChange).toHaveBeenLastCalledWith(
      "[Read](https://web.dev/articles/vitals) why links matter.\n",
    );
  });

  it("puts a bare wikilink at the cursor when nothing is selected", async () => {
    const { onChange } = linking(23, 23);
    expect(screen.getByRole("form", { name: "Add a link" })).toBeInTheDocument();
    await screen.findByRole("button", { name: /Why internal links matter/ });
    fireEvent.submit(screen.getByRole("form"));
    expect(onChange).toHaveBeenLastCalledWith(`${POST}[[why-internal-links-matter]]\n`);
  });

  it("closes with Escape and changes nothing", () => {
    const { onChange } = linking(6, 22);
    fireEvent.keyDown(screen.getByLabelText("Find a post, or paste a web address"), {
      key: "Escape",
    });
    expect(screen.queryByRole("form")).not.toBeInTheDocument();
    expect(onChange).not.toHaveBeenCalled();
  });
});
