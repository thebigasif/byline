import { describe, expect, it } from "vitest";
import {
  emptyGrid,
  readTable,
  withCell,
  withColumn,
  withoutColumn,
  withoutRow,
  withRow,
  writeTable,
} from "./table";

const BLOCKS = [
  "| Block | What it adds |",
  "| --- | :---: |",
  "| Callout | A tinted box for a tip |",
  "| FAQ | Questions and answers |",
].join("\n");

describe("a markdown table as a grid", () => {
  it("reads the headings, how each column lines up, and the rows", () => {
    expect(readTable(BLOCKS)).toEqual({
      head: ["Block", "What it adds"],
      align: [null, "center"],
      rows: [
        ["Callout", "A tinted box for a tip"],
        ["FAQ", "Questions and answers"],
      ],
    });
  });

  it("writes a standard table that reads back the same", () => {
    const grid = readTable(BLOCKS);
    expect(grid).not.toBeNull();
    if (grid === null) {
      return;
    }
    expect(writeTable(grid)).toBe(BLOCKS);
    const written = writeTable({ ...grid, align: ["left", "right"] });
    expect(written.split("\n")[1]).toBe("| :--- | ---: |");
    expect(readTable(written)?.align).toEqual(["left", "right"]);
  });

  it("reads tables written without outer pipes, short rows and escaped pipes", () => {
    const loose = "Name | Price\n--- | ---:\nA \\| B | 4\nC";
    expect(readTable(loose)).toEqual({
      head: ["Name", "Price"],
      align: [null, "right"],
      rows: [
        ["A \\| B", "4"],
        ["C", ""],
      ],
    });
  });

  it("refuses what is not a table it can hold", () => {
    expect(readTable("Just a sentence.")).toBeNull();
    expect(readTable("| a | b |\n| c | d |")).toBeNull();
    expect(readTable("| a | b |\n| --- |")).toBeNull();
  });

  it("keeps a cell on one line and escapes the pipes typed into it", () => {
    const grid = withCell(emptyGrid(), 0, 1, "fast | small\nand light");
    expect(writeTable(withCell(grid, -1, 0, "Name"))).toBe(
      "| Name |  |\n| --- | --- |\n|  | fast \\| small and light |",
    );
  });

  it("adds and removes rows and columns at the end, keeping one of each", () => {
    let grid = withColumn(withRow(emptyGrid()));
    expect(grid.head).toHaveLength(3);
    expect(grid.rows).toEqual([
      ["", "", ""],
      ["", "", ""],
    ]);
    grid = withoutRow(withoutRow(withoutRow(grid)));
    grid = withoutColumn(withoutColumn(withoutColumn(grid)));
    expect(grid).toEqual({ head: [""], align: [null], rows: [[""]] });
  });
});
