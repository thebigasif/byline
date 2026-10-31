import { EditorContent, useEditor, useEditorState } from "@tiptap/react";
import { useRef } from "react";
import { en } from "../strings/en";
import { SlashMenu } from "./aids/SlashMenu";
import { openTrigger } from "./aids/trigger";
import { type KeyHandler, noKeys } from "./aids/useMenu";
import { extensions, fileText } from "./richText";

type Props = {
  markdown: string;
  onChange: (markdown: string) => void;
  onBlur: () => void;
};

/** The post's text as formatted text, read from and written to markdown. */
export function RichText({ markdown, onChange, onBlur }: Props) {
  // The open menu takes the arrow keys, Enter and Escape before the text does.
  const keys = useRef<KeyHandler>(noKeys);
  const editor = useEditor({
    extensions,
    content: markdown,
    contentType: "markdown",
    // The admin's policy allows only its own stylesheet, so no style tag.
    injectCSS: false,
    editorProps: {
      attributes: {
        class: "text",
        role: "textbox",
        "aria-multiline": "true",
        "aria-label": en.editor.text,
      },
      handleKeyDown: (_view, event) => keys.current(event),
    },
    onUpdate: ({ editor: current }) => onChange(fileText(current.getMarkdown())),
    onBlur,
  });
  const trigger = useEditorState({
    editor,
    selector: (snapshot) => openTrigger(snapshot.editor.state),
  });
  return (
    <div className="writing">
      <EditorContent editor={editor} />
      {trigger?.kind === "slash" ? (
        <SlashMenu
          key={`${trigger.from}:${trigger.query}`}
          editor={editor}
          trigger={trigger}
          keys={keys}
        />
      ) : null}
    </div>
  );
}
