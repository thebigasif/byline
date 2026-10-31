// The words for editing a table as a grid, beside en.ts, which is near its size limit.
export const tables = {
  title: "Table",
  hint: "The top row holds the headings. Each row below is one line of the table.",
  cells: "Table cells",
  heading: (column: number) => `Heading ${column}`,
  cell: (row: number, column: number) => `Row ${row}, column ${column}`,
  addRow: "Add a row",
  addColumn: "Add a column",
  removeRow: "Remove the last row",
  removeColumn: "Remove the last column",
  save: "Save the table",
  cancel: "Cancel",
  slashHint: "Rows and columns you fill in as a grid",
};
