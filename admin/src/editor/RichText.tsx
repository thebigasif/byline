import { EditorContent, useEditor, useEditorState } from "@tiptap/react";
import { type ReactNode, useRef, useState } from "react";
import { en } from "../strings/en";
import { BlockForm } from "./aids/BlockForm";
import { applyDetails, type Details, detailsOf, selectedDetails } from "./aids/blockForm";
import { SlashMenu } from "./aids/SlashMenu";
import { openTrigger } from "./aids/trigger";
import { type KeyHandler, menuKey, noKeys } from "./aids/useMenu";
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
  const [details, setDetails] = useState<Details | null>(null);
  const opened = (found: Details | null) => {
    if (found !== null) {
      setDetails(found);
    }
    return found !== null;
  };
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
      handleKeyDown: (view, event) => {
        if (keys.current(event)) {
          return true;
        }
        // Enter on a selected block card opens its details.
        const selected = menuKey(event) === "pick" ? selectedDetails(view.state) : null;
        return opened(selected);
      },
      handleDoubleClickOn: (_view, _pos, node, nodePos) => opened(detailsOf(node, nodePos)),
    },
    onUpdate: ({ editor: current }) => onChange(fileText(current.getMarkdown())),
    onBlur,
  });
  const trigger = useEditorState({
    editor,
    selector: (snapshot) => openTrigger(snapshot.editor.state),
  });
  const closeForm = () => {
    setDetails(null);
    editor.commands.focus();
  };
  let aid: ReactNode = null;
  if (details !== null) {
    aid = (
      <BlockForm
        key={details.at}
        details={details}
        onSave={(values) => {
          applyDetails(editor, details, values);
          setDetails(null);
        }}
        onCancel={closeForm}
      />
    );
  } else if (trigger?.kind === "slash") {
    aid = (
      <SlashMenu
        key={`${trigger.from}:${trigger.query}`}
        editor={editor}
        trigger={trigger}
        keys={keys}
        onDetails={setDetails}
      />
    );
  }
  return (
    <div className="writing">
      <EditorContent editor={editor} />
      {aid}
    </div>
  );
}
