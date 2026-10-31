import { Editor } from "@tiptap/core";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { RichText } from "../src/editor/RichText";

/** The rich-text view with `markdown`, the editor behind it and what it saved. */
export function writing(markdown = "") {
  const onChange = vi.fn();
  render(<RichText markdown={markdown} onChange={onChange} onBlur={vi.fn()} />);
  const text: unknown = screen.getByRole("textbox");
  if (!(text instanceof HTMLElement && "editor" in text && text.editor instanceof Editor)) {
    throw new Error("Tiptap keeps its editor on the text box");
  }
  const editor = text.editor;
  const type = (words: string) => act(() => void editor.commands.insertContent(words));
  const press = (key: string, modifiers: { ctrlKey?: boolean; shiftKey?: boolean } = {}) =>
    act(() => void fireEvent.keyDown(text, { key, ...modifiers }));
  return { editor, text, type, press, onChange };
}
