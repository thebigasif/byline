import { act, fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { writing } from "../../../test/editor";
import { axeViolations } from "../../../test/render";
import { en } from "../../strings/en";
import { BLOCK_SPECS } from "../blocks/specs";
import { fieldsOf } from "./blockForm";

const fill = (label: string, value: string) =>
  fireEvent.change(screen.getByLabelText(label), { target: { value } });

describe("a block's details", () => {
  it("has a label for every detail of every block", () => {
    const labels: Record<string, string> = en.fields;
    for (const spec of BLOCK_SPECS) {
      for (const field of fieldsOf(spec)) {
        expect(labels[field], `${spec.name} ${field}`).toBeDefined();
      }
    }
  });

  it("are asked for before a block with details goes in", async () => {
    const { type, press, onChange } = writing();
    type("/button");
    press("Enter");
    expect(screen.getByRole("form", { name: "Button" })).toBeInTheDocument();
    expect(screen.getByLabelText("Link address")).toHaveFocus();
    expect(await axeViolations()).toEqual([]);
    fill("Link address", "/pricing/");
    fill("Label, the words readers see on it", "See prices");
    fireEvent.click(screen.getByRole("button", { name: "Add the block" }));
    expect(onChange).toHaveBeenLastCalledWith(
      expect.stringMatching(/^::button\{href="\/pricing\/" label="See prices"\}\n/),
    );
    expect(screen.queryByRole("form")).not.toBeInTheDocument();
  });

  it("write a block's kind and its text", () => {
    const { type, press, onChange } = writing();
    type("/callout");
    press("Enter");
    fill("Kind of note", "tip");
    fill("Text inside the block, in markdown", "Back up first.\n");
    fireEvent.click(screen.getByRole("button", { name: "Add the block" }));
    expect(onChange).toHaveBeenLastCalledWith(
      expect.stringMatching(/^:::callout\{kind=tip\}\nBack up first\.\n:::\n/),
    );
  });

  it("add nothing when the writer cancels", () => {
    const { editor, type, press } = writing();
    type("/figure");
    press("Enter");
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("form")).not.toBeInTheDocument();
    expect(editor.getText()).toBe("");
    expect(editor.getJSON().content?.[0]?.type).toBe("paragraph");
  });

  it("open with Enter on a selected block and change it", () => {
    const { editor, press, onChange } = writing('::button{href="/pricing/" label="See prices"}');
    act(() => void editor.commands.setNodeSelection(0));
    press("Enter");
    expect(screen.getByLabelText("Link address")).toHaveValue("/pricing/");
    fill("Label, the words readers see on it", "Compare plans");
    fireEvent.click(screen.getByRole("button", { name: "Save the details" }));
    expect(onChange).toHaveBeenLastCalledWith(
      expect.stringMatching(/^::button\{href="\/pricing\/" label="Compare plans"\}\n/),
    );
  });
});
