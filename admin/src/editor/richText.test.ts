import { describe, expect, it } from "vitest";
import { fileText, fitsRichText, throughRichText } from "./richText";

// One sample per block the rich-text view holds. Each must come back
// byte for byte, so opening and saving a post changes nothing.
const BLOCKS: Record<string, string> = {
  paragraph: "A topic cluster is a group of posts on one subject.",
  heading: "## Plan it on one page",
  bold: "Pick the subject **your customers** ask about most.",
  italic: "Write the *short* posts first.",
  link: "Read [why internal links matter](/blog/why-internal-links-matter/).",
  code: "Run `khalid lint` before you publish.",
  bulletList: "- Feed the starter\n- Fold the dough",
  orderedList: "1. Pick the subject\n2. Write the main question",
  quote: "> Write the short posts first.",
  codeBlock: "```sh\nkhalid serve\n```",
  divider: "One part.\n\n---\n\nAnother part.",
  wikilink: "Read [[why-internal-links-matter]] next.",
  wikilinkWords: "Read [[why-internal-links-matter|the case for links]] next.",
  wikilinkTitle: "Read [[why internal links matter]] next.",
};

describe("the rich-text view", () => {
  for (const [block, markdown] of Object.entries(BLOCKS)) {
    it(`gives back a ${block} unchanged`, () => {
      expect(throughRichText(markdown).trimEnd()).toBe(markdown);
      expect(fitsRichText(markdown)).toBe(true);
    });
  }

  it("ends a saved file in exactly one line break", () => {
    expect(fileText("Hello\n\n")).toBe("Hello\n");
    expect(fileText("")).toBe("");
  });
});
