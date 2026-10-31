import type { Editor } from "@tiptap/core";
import type { Node as ProseMirrorNode } from "@tiptap/pm/model";
import { type EditorState, NodeSelection } from "@tiptap/pm/state";
import { emptyGrid, type Grid, readTable, writeTable } from "../blocks/table";

/** The node a markdown table is kept in. */
const TABLE_NODE = "markdownTable";

/** A table in the grid: a new one to add at `at`, or the one there. */
export type TableEdit = {
  grid: Grid;
  at: number;
  editing: boolean;
};

/** An empty table to fill in, added at `at` once saved. */
export function newTable(at: number): TableEdit {
  return { grid: emptyGrid(), at, editing: false };
}

/** The table `node`, sitting at `at`, as a grid, if it is one the grid can hold. */
export function tableOf(node: ProseMirrorNode, at: number): TableEdit | null {
  const source: unknown = node.attrs.source;
  if (node.type.name !== TABLE_NODE || typeof source !== "string") {
    return null;
  }
  const grid = readTable(source);
  return grid === null ? null : { grid, at, editing: true };
}

/** The table selected in the text, if one is. */
export function selectedTable(state: EditorState): TableEdit | null {
  const { selection } = state;
  return selection instanceof NodeSelection ? tableOf(selection.node, selection.from) : null;
}

/** Adds the table the grid holds, or writes it over the table it was opened on. */
export function applyTable(editor: Editor, edit: TableEdit, grid: Grid) {
  const source = writeTable(grid);
  if (!edit.editing) {
    editor
      .chain()
      .focus()
      .insertContentAt(edit.at, { type: TABLE_NODE, attrs: { source } })
      .run();
    return;
  }
  editor
    .chain()
    .focus()
    .command(({ tr }) => {
      const node = tr.doc.nodeAt(edit.at);
      if (node?.type.name !== TABLE_NODE) {
        return false;
      }
      tr.setNodeMarkup(edit.at, undefined, { ...node.attrs, source });
      return true;
    })
    .run();
}
