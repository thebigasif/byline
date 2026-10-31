import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { afterEach, describe, expect, it } from "vitest";
import { aidsKey, CLOSE, findTrigger, openTrigger, WritingAids } from "./trigger";

describe("finding what the writer typed", () => {
  it("opens the block menu for a slash on an empty line", () => {
    expect(findTrigger("/")).toEqual({ kind: "slash", query: "", length: 1 });
    expect(findTrigger("/call")).toEqual({ kind: "slash", query: "call", length: 5 });
  });

  it("leaves a slash inside a sentence alone", () => {
    expect(findTrigger("Half is 1/2")).toBeNull();
    expect(findTrigger("See example.com/blog")).toBeNull();
    expect(findTrigger("/two words")).toBeNull();
  });

  it("opens the post menu for an at sign after a space", () => {
    expect(findTrigger("Read @inter")).toEqual({ kind: "mention", query: "inter", length: 6 });
    expect(findTrigger("@")).toEqual({ kind: "mention", query: "", length: 1 });
  });

  it("leaves an email address alone", () => {
    expect(findTrigger("Write to hello@example")).toBeNull();
  });

  it("opens the wikilink menu after two brackets", () => {
    expect(findTrigger("See [[topic clu")).toEqual({
      kind: "wikilink",
      query: "topic clu",
      length: 11,
    });
  });

  it("closes the wikilink menu once the link is written", () => {
    expect(findTrigger("See [[topic-clusters]]")).toBeNull();
    expect(findTrigger("See [[topic-clusters|the guide")).toBeNull();
  });

  it("opens the emoji menu for a colon and two letters after a space", () => {
    expect(findTrigger("We shipped it :tad")).toEqual({ kind: "emoji", query: "tad", length: 4 });
    expect(findTrigger(":+1")).toEqual({ kind: "emoji", query: "+1", length: 3 });
  });

  it("leaves times, addresses, block lines and a lone colon alone", () => {
    expect(findTrigger("Doors open at 10:30")).toBeNull();
    expect(findTrigger("See https://example")).toBeNull();
    expect(findTrigger("::callout")).toBeNull();
    expect(findTrigger("Note: t")).toBeNull();
    expect(findTrigger("Wait :t")).toBeNull();
  });
});

describe("the writing aids in the editor", () => {
  let editor: Editor | null = null;
  afterEach(() => editor?.destroy());

  function typed(text: string): Editor {
    editor = new Editor({ extensions: [StarterKit, WritingAids], content: "<p></p>" });
    editor.commands.insertContent(text);
    return editor;
  }

  it("knows the open menu and where its text sits", () => {
    const open = typed("/call");
    expect(openTrigger(open.state)).toEqual({ kind: "slash", query: "call", from: 1, to: 6 });
  });

  it("opens nothing in a code block", () => {
    editor = new Editor({
      extensions: [StarterKit, WritingAids],
      content: "<pre><code></code></pre>",
    });
    editor.commands.insertContent("/call");
    expect(openTrigger(editor.state)).toBeNull();
  });

  it("stays closed after Escape until a new trigger is typed", () => {
    const open = typed("Read @inter");
    open.view.dispatch(open.state.tr.setMeta(aidsKey, CLOSE));
    expect(openTrigger(open.state)).toBeNull();
    open.commands.insertContent("nal");
    expect(openTrigger(open.state)).toBeNull();
    open.commands.insertContent(" @");
    expect(openTrigger(open.state)?.kind).toBe("mention");
  });
});
