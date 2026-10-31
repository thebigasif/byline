import { act, fireEvent, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { writing } from "../../../test/editor";
import { axeViolations } from "../../../test/render";
import { tables as words } from "../../strings/tables";

const PRICES = "| Plan | Price |\n| --- | ---: |\n| Starter | 9 |";

const fill = (label: string, value: string) =>
  fireEvent.change(screen.getByLabelText(label), { target: { value } });

describe("a table as a grid", () => {
  it("opens with Enter on a selected table, cell by cell, and writes it back", async () => {
    const { editor, press, onChange } = writing(PRICES);
    act(() => void editor.commands.setNodeSelection(0));
    press("Enter");
    const grid = screen.getByRole("table", { name: words.cells });
    expect(within(grid).getByLabelText(words.heading(1))).toHaveFocus();
    expect(within(grid).getByLabelText(words.cell(1, 2))).toHaveValue("9");
    expect(await axeViolations()).toEqual([]);
    fill(words.cell(1, 2), "12");
    fireEvent.click(screen.getByRole("button", { name: words.addRow }));
    fill(words.cell(2, 1), "Team");
    fill(words.cell(2, 2), "29");
    fireEvent.click(screen.getByRole("button", { name: words.save }));
    const saved = "| Plan | Price |\n| --- | ---: |\n| Starter | 12 |\n| Team | 29 |";
    expect(onChange).toHaveBeenLastCalledWith(expect.stringContaining(saved));
    expect(screen.queryByRole("form")).not.toBeInTheDocument();
  });

  it("adds a new table from the slash menu once its cells are in", () => {
    const { type, press, onChange } = writing();
    type("/table");
    press("Enter");
    expect(screen.getByRole("form", { name: words.title })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: words.addColumn }));
    fill(words.heading(1), "Block");
    fill(words.heading(2), "Form");
    fill(words.heading(3), "Kept as");
    fill(words.cell(1, 1), "toc");
    fireEvent.click(screen.getByRole("button", { name: words.save }));
    const saved = "| Block | Form | Kept as |\n| --- | --- | --- |\n| toc |  |  |";
    expect(onChange).toHaveBeenLastCalledWith(expect.stringContaining(saved));
  });

  it("changes nothing when the writer cancels", () => {
    const { editor, press, onChange } = writing(PRICES);
    act(() => void editor.commands.setNodeSelection(0));
    press("Enter");
    fill(words.cell(1, 1), "Pro");
    fireEvent.click(screen.getByRole("button", { name: words.cancel }));
    expect(screen.queryByRole("form")).not.toBeInTheDocument();
    // Selecting the table adds the empty last line the editor keeps after a
    // block, so it may report the text once, but never different text.
    for (const [text] of onChange.mock.calls) {
      expect(text).toBe(`${PRICES}\n`);
    }
  });
});
