import { useEffect, useRef, useState } from "react";
import { tables as words } from "../../strings/tables";
import {
  type Grid,
  withCell,
  withColumn,
  withoutColumn,
  withoutRow,
  withRow,
} from "../blocks/table";

type Props = {
  grid: Grid;
  onSave: (grid: Grid) => void;
  onCancel: () => void;
};

type CellsProps = {
  cells: string[];
  row: number;
  onCell: (column: number, text: string) => void;
};

/** One line of the grid: a box to type in for each cell; row -1 holds the headings. */
function Cells({ cells, row, onCell }: CellsProps) {
  const Cell = row < 0 ? "th" : "td";
  const columns = cells.map((text, column) => ({ text, column, id: `cell-${row}-${column}` }));
  return (
    <tr>
      {columns.map(({ text, column, id }) => (
        <Cell key={id}>
          <input
            aria-label={row < 0 ? words.heading(column + 1) : words.cell(row + 1, column + 1)}
            value={text}
            onChange={(event) => onCell(column, event.target.value)}
          />
        </Cell>
      ))}
    </tr>
  );
}

/** A table as a grid of cells to type in, with rows and columns added or removed at the end. */
export function TableGrid({ grid: start, onSave, onCancel }: Props) {
  const [grid, setGrid] = useState(start);
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => {
    form.current?.querySelector<HTMLElement>("input")?.focus();
  }, []);
  const cell = (row: number) => (column: number, text: string) =>
    setGrid((current) => withCell(current, row, column, text));
  const rows = grid.rows.map((cells, row) => ({ cells, row, id: `row-${row}` }));
  return (
    <form
      ref={form}
      className="focus wide block-form"
      aria-labelledby="table-grid-title"
      onSubmit={(event) => {
        event.preventDefault();
        onSave(grid);
      }}
    >
      <h2 id="table-grid-title">{words.title}</h2>
      <p className="hint">{words.hint}</p>
      <div className="table-grid">
        <table aria-label={words.cells}>
          <thead>
            <Cells cells={grid.head} row={-1} onCell={cell(-1)} />
          </thead>
          <tbody>
            {rows.map(({ cells, row, id }) => (
              <Cells key={id} cells={cells} row={row} onCell={cell(row)} />
            ))}
          </tbody>
        </table>
      </div>
      <div className="flex flex-wrap gap-3 mb-4">
        <button type="button" className="btn" onClick={() => setGrid(withRow)}>
          {words.addRow}
        </button>
        <button type="button" className="btn" onClick={() => setGrid(withColumn)}>
          {words.addColumn}
        </button>
        <button type="button" className="btn" onClick={() => setGrid(withoutRow)}>
          {words.removeRow}
        </button>
        <button type="button" className="btn" onClick={() => setGrid(withoutColumn)}>
          {words.removeColumn}
        </button>
      </div>
      <div className="flex flex-wrap gap-3">
        <button type="submit" className="btn primary">
          {words.save}
        </button>
        <button type="button" className="btn" onClick={onCancel}>
          {words.cancel}
        </button>
      </div>
    </form>
  );
}
