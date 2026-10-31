import { Editor, type JSONContent } from "@tiptap/core";
import { describe, expect, it } from "vitest";
import { extensions } from "../richText";
import { wikilinkMarkdown, wikilinkWords } from "./wikilink";

/** The inline nodes of the first paragraph of `markdown`. */
function inline(markdown: string): JSONContent[] {
  const editor = new Editor({ extensions, content: markdown, contentType: "markdown" });
  const nodes = editor.getJSON().content?.[0]?.content ?? [];
  editor.destroy();
  return nodes;
}

describe("a wikilink", () => {
  it("reads as its own node with its slug", () => {
    expect(inline("See [[topic-clusters]].")[1]).toEqual({
      type: "wikilink",
      attrs: { slug: "topic-clusters", text: null },
    });
  });

  it("keeps the words after the pipe", () => {
    expect(inline("See [[topic-clusters|the guide]].")[1]?.attrs).toEqual({
      slug: "topic-clusters",
      text: "the guide",
    });
  });

  it("shows its words in the text", () => {
    const editor = new Editor({
      extensions,
      content: "See [[topic-clusters|the guide]].",
      contentType: "markdown",
    });
    expect(editor.getText()).toBe("See the guide.");
    editor.destroy();
  });

  it("leaves brackets that are not a wikilink as text", () => {
    const types = inline("Arrays look like [[1, 2], [3]] in JSON.").map((node) => node.type);
    expect(types).not.toContain("wikilink");
  });

  it("drops words that would end the wikilink early", () => {
    expect(wikilinkWords("SEO in [2026]")).toBeNull();
    expect(wikilinkWords("")).toBeNull();
    expect(wikilinkWords("the guide")).toBe("the guide");
  });

  it("writes the words only when there are some", () => {
    expect(wikilinkMarkdown("title-tags", null)).toBe("[[title-tags]]");
    expect(wikilinkMarkdown("title-tags", "a title tag")).toBe("[[title-tags|a title tag]]");
  });
});
