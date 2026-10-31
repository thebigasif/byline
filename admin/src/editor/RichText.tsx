import { EditorContent, useEditor, useEditorState } from "@tiptap/react";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { en } from "../strings/en";
import { BlockForm } from "./aids/BlockForm";
import { applyDetails, type Details, detailsOf, selectedDetails } from "./aids/blockForm";
import { EmojiMenu } from "./aids/EmojiMenu";
import { LinkSearch } from "./aids/LinkSearch";
import { type Chosen, chosenIn, isLinkKey, linkToAddress, linkToPost } from "./aids/linkTo";
import { type Finding, PictureAid } from "./aids/PictureAid";
import { PostMenu } from "./aids/PostMenu";
import { SaveSnippet } from "./aids/SaveSnippet";
import { ShortcutHelp } from "./aids/ShortcutHelp";
import { SlashMenu } from "./aids/SlashMenu";
import { placeSnippet, selectedMarkdown } from "./aids/snippetBlock";
import { TableGrid } from "./aids/TableGrid";
import { applyTable, newTable, selectedTable, type TableEdit, tableOf } from "./aids/tableEdit";
import { openTrigger } from "./aids/trigger";
import { isHelpKey, type KeyHandler, menuKey, noKeys } from "./aids/useMenu";
import { extensions, fileText } from "./richText";

type Props = {
  markdown: string;
  /** Shows the text without letting the writer change it, while someone else holds the post. */
  readOnly?: boolean;
  onChange: (markdown: string) => void;
  onBlur: () => void;
};

/** The post's text as formatted text, read from and written to markdown. */
export function RichText({ markdown, readOnly = false, onChange, onBlur }: Props) {
  // The open menu takes the arrow keys, Enter and Escape before the text does.
  const keys = useRef<KeyHandler>(noKeys);
  const [details, setDetails] = useState<Details | null>(null);
  const [linking, setLinking] = useState<Chosen | null>(null);
  const [help, setHelp] = useState(false);
  // The selected blocks in markdown while the writer saves them as a snippet.
  const [snippet, setSnippet] = useState<string | null>(null);
  const opened = (found: Details | null) => {
    if (found !== null) {
      setDetails(found);
    }
    return found !== null;
  };
  // Photo or GIF search or the media library, and where the picked picture goes.
  const [finding, setFinding] = useState<Finding | null>(null);
  // A table being edited as a grid of cells.
  const [table, setTable] = useState<TableEdit | null>(null);
  const openedTable = (found: TableEdit | null) => {
    if (found !== null) {
      setTable(found);
    }
    return found !== null;
  };
  const editor = useEditor({
    extensions,
    content: markdown,
    contentType: "markdown",
    editable: !readOnly,
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
        if (isLinkKey(event)) {
          setLinking(chosenIn(view.state));
          return true;
        }
        if (isHelpKey(event)) {
          setHelp((shown) => !shown);
          return true;
        }
        // Enter on a selected block card opens its details, or its grid for a table.
        if (menuKey(event) !== "pick" || !view.editable) {
          return false;
        }
        return opened(selectedDetails(view.state)) || openedTable(selectedTable(view.state));
      },
      handleDoubleClickOn: (view, _pos, node, nodePos) =>
        view.editable && (opened(detailsOf(node, nodePos)) || openedTable(tableOf(node, nodePos))),
    },
    onUpdate: ({ editor: current }) => onChange(fileText(current.getMarkdown())),
    onBlur,
  });
  useEffect(() => {
    // No update event: locking the text changes nothing in it.
    editor.setEditable(!readOnly, false);
  }, [editor, readOnly]);
  const trigger = useEditorState({
    editor,
    selector: (snapshot) => openTrigger(snapshot.editor.state),
  });
  const selecting = useEditorState({
    editor,
    selector: (snapshot) => !snapshot.editor.state.selection.empty,
  });
  const closeForm = () => {
    setDetails(null);
    setLinking(null);
    setHelp(false);
    setSnippet(null);
    setTable(null);
    setFinding(null);
    editor.commands.focus();
  };
  let aid: ReactNode = null;
  if (finding !== null) {
    aid = (
      <PictureAid
        editor={editor}
        finding={finding}
        onPlaced={() => setFinding(null)}
        onCancel={closeForm}
      />
    );
  } else if (table !== null) {
    aid = (
      <TableGrid
        key={table.at}
        grid={table.grid}
        onSave={(grid) => {
          applyTable(editor, table, grid);
          setTable(null);
        }}
        onCancel={closeForm}
      />
    );
  } else if (snippet !== null) {
    aid = (
      <SaveSnippet
        markdown={snippet}
        onSaved={(slug) => {
          placeSnippet(editor, slug);
          setSnippet(null);
        }}
        onCancel={closeForm}
      />
    );
  } else if (details !== null) {
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
  } else if (linking !== null) {
    aid = (
      <LinkSearch
        chosen={linking}
        onPost={(post) => {
          linkToPost(editor, linking, post);
          setLinking(null);
        }}
        onAddress={(address) => {
          linkToAddress(editor, linking, address);
          setLinking(null);
        }}
        onCancel={closeForm}
      />
    );
  } else if (help) {
    aid = <ShortcutHelp onClose={closeForm} />;
  } else if (trigger?.kind === "emoji") {
    const key = `${trigger.kind}:${trigger.from}`;
    aid = <EmojiMenu key={key} editor={editor} trigger={trigger} keys={keys} />;
  } else if (trigger !== null && trigger.kind !== "slash") {
    const key = `${trigger.kind}:${trigger.from}`;
    aid = <PostMenu key={key} editor={editor} trigger={trigger} keys={keys} />;
  } else if (trigger?.kind === "slash") {
    aid = (
      <SlashMenu
        key={`${trigger.from}:${trigger.query}`}
        editor={editor}
        trigger={trigger}
        keys={keys}
        onDetails={setDetails}
        onTable={(at) => setTable(newTable(at))}
        onPictures={(source, at) => setFinding({ source, at })}
      />
    );
  }
  return (
    <div className="writing">
      <EditorContent editor={editor} />
      {aid === null && selecting && !readOnly ? (
        <button
          type="button"
          className="btn mt-3"
          onClick={() => setSnippet(selectedMarkdown(editor))}
        >
          {en.snippets.open}
        </button>
      ) : null}
      {aid}
    </div>
  );
}
