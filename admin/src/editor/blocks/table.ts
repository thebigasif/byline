// A markdown table as a grid of cells, so the editor can show one as boxes
// to type in and write it back as a standard table (plan 4.6, KH-EDT-1).

/** How a column lines up its text, from the dashes under the headings. */
export type Align = "left" | "center" | "right" | null;

/** A table: the headings, how each column lines up, and the rows below. */
export type Grid = {
  head: string[];
  align: Align[];
  rows: string[][];
};

/** The cells of one table line, split on pipes that are not written `\|`. */
function cells(line: string): string[] {
  let inner = line.trim();
  inner = inner.startsWith("|") ? inner.slice(1) : inner;
  inner = inner.endsWith("|") && !inner.endsWith("\\|") ? inner.slice(0, -1) : inner;
  return inner.split(/(?<!\\)\|/).map((cell) => cell.trim());
}

/** The line under the headings: dashes per column, with colons for the side it lines up on. */
function alignments(line: string): Align[] | null {
  const marks = cells(line);
  if (!marks.every((mark) => /^:?-+:?$/.test(mark))) {
    return null;
  }
  return marks.map((mark) => {
    const left = mark.startsWith(":");
    const right = mark.endsWith(":");
    if (left && right) {
      return "center";
    }
    if (right) {
      return "right";
    }
    return left ? "left" : null;
  });
}

/** `row` with exactly `width` cells: short rows filled with empty cells, long ones cut. */
function fitted(row: string[], width: number): string[] {
  return Array.from({ length: width }, (_, n) => row[n] ?? "");
}

/** The grid in a markdown table, or `null` when `source` is not one the grid can hold. */
export function readTable(source: string): Grid | null {
  const lines = source.trim().split("\n");
  const [first, second, ...rest] = lines;
  if (first === undefined || second === undefined || !first.includes("|")) {
    return null;
  }
  const head = cells(first);
  const align = alignments(second);
  if (align === null || align.length !== head.length) {
    return null;
  }
  const rows = rest.map((line) => fitted(cells(line), head.length));
  return { head, align, rows };
}

/** A cell as one table line holds it: on one line, with its pipes escaped. */
function written(cell: string): string {
  return cell
    .replace(/\s*\n\s*/g, " ")
    .replace(/(?<!\\)\|/g, "\\|")
    .trim();
}

function line(row: string[]): string {
  return `| ${row.map(written).join(" | ")} |`;
}

function mark(align: Align): string {
  switch (align) {
    case "left":
      return ":---";
    case "center":
      return ":---:";
    case "right":
      return "---:";
    default:
      return "---";
  }
}

/** The grid as a standard markdown table. */
export function writeTable(grid: Grid): string {
  const under = `| ${grid.align.map(mark).join(" | ")} |`;
  return [line(grid.head), under, ...grid.rows.map(line)].join("\n");
}

/** A new table of empty cells, two columns by two rows, for the writer to fill in. */
export function emptyGrid(): Grid {
  return { head: ["", ""], align: [null, null], rows: [["", ""]] };
}

/** The grid with an empty row at the end. */
export function withRow(grid: Grid): Grid {
  return { ...grid, rows: [...grid.rows, grid.head.map(() => "")] };
}

/** The grid with an empty column at the end. */
export function withColumn(grid: Grid): Grid {
  return {
    head: [...grid.head, ""],
    align: [...grid.align, null],
    rows: grid.rows.map((row) => [...row, ""]),
  };
}

/** The grid without its last row; a table keeps at least one. */
export function withoutRow(grid: Grid): Grid {
  return grid.rows.length <= 1 ? grid : { ...grid, rows: grid.rows.slice(0, -1) };
}

/** The grid without its last column; a table keeps at least one. */
export function withoutColumn(grid: Grid): Grid {
  if (grid.head.length <= 1) {
    return grid;
  }
  return {
    head: grid.head.slice(0, -1),
    align: grid.align.slice(0, -1),
    rows: grid.rows.map((row) => row.slice(0, -1)),
  };
}

/** The grid with `text` in one cell: row -1 is the headings. */
export function withCell(grid: Grid, row: number, column: number, text: string): Grid {
  const put = (cellsOf: string[]) => cellsOf.map((cell, n) => (n === column ? text : cell));
  if (row < 0) {
    return { ...grid, head: put(grid.head) };
  }
  return { ...grid, rows: grid.rows.map((cellsOf, n) => (n === row ? put(cellsOf) : cellsOf)) };
}
