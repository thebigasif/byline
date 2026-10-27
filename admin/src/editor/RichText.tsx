import { EditorContent, useEditor } from "@tiptap/react";
import { en } from "../strings/en";
import { extensions, fileText } from "./richText";

type Props = {
  markdown: string;
  onChange: (markdown: string) => void;
  onBlur: () => void;
};

/** The post's text as formatted text, read from and written to markdown. */
export function RichText({ markdown, onChange, onBlur }: Props) {
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
    },
    onUpdate: ({ editor: current }) => onChange(fileText(current.getMarkdown())),
    onBlur,
  });
  return <EditorContent editor={editor} />;
}
