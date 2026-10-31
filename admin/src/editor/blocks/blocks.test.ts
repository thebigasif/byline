import { Editor, type JSONContent } from "@tiptap/core";
import { describe, expect, it } from "vitest";
import { extensions, fitsRichText, throughRichText } from "../richText";
import { BLOCK_SPECS } from "./specs";

// One sample per plan 4.6 block beyond the starter kit, written the way
// `khalid-content` reads it. Each must come back byte for byte, so opening
// and saving a post with any block changes nothing in its file.
const BLOCKS: Record<string, string> = {
  audio: '::audio{src="/media/episode-12.mp3" title="Episode 12: internal links"}',
  bookmark:
    '::bookmark{href="https://web.dev/articles/vitals" title="Web Vitals" description="Quality signals for a good experience." image="https://web.dev/vitals.png"}',
  business:
    '::business{name="Field Notes Studio" type="ProfessionalService" street="12 Mill Lane" locality="Leeds" postcode="LS1 4AB" country="GB" phone="+44 113 496 0000" email="hello@example.com" href="https://www.example.com"}',
  button: '::button{href="/pricing/" label="See prices"}',
  callout:
    ":::callout{kind=tip}\nLink from new posts to old ones, then go back the other way.\n:::",
  embed: '::embed{src="https://www.youtube.com/watch?v=dQw4w9WgXcQ" title="How search works"}',
  faq: '::::faq\n:::question{text="Does it work offline?"}\nYes, every page works with scripts off.\n:::\n::::',
  figure:
    ':::figure{src="shelf.jpg" alt="A bakery shelf of sourdough loaves"}\nThe morning bake.\n:::',
  file: '::file{src="/media/link-audit.csv" title="The link audit sheet"}',
  form: ':::form{id="contact" button="Send"}\n- email reply_to: Your email address\n:::',
  gallery:
    ":::gallery\n![The editor](/media/editor.png)\n![The posts table](/media/posts.png)\n:::",
  gated:
    ':::gated{note="The rest is for subscribers." label="Email" button="Read"}\nThe full list.\n:::',
  header: ':::header{image="/media/cover.jpg" alt="A desk by a window"}\n# Writing that ranks\n:::',
  howto:
    '::::howto{name="Add a link" time="2:00"}\n:::step{name="Open the post"}\nFind it in Posts.\n:::\n::::',
  html: ':::html{show="visitors"}\n<p>Sign in to comment.</p>\n:::',
  maths: ':::maths{label="Compound growth"}\nA = P(1 + r)^t\n:::',
  newsletter: '::newsletter{label="Your email address" button="Get the weekly links"}',
  product:
    ':::product{name="Kit" image="/media/kit.jpg" price="49.00" currency="USD" rating="4.5" href="/kit/" button="Buy the kit"}\nEverything to start.\n:::',
  recipe:
    '::::recipe{name="Toast" servings="1"}\n:::ingredients\n- Bread\n:::\n:::method\n1. Toast it\n:::\n::::',
  review: ':::review{item="A book" type="Book" rating="4" author="Asif"}\nClear and short.\n:::',
  toc: "::toc",
  toggle: ':::toggle{summary="Why does this matter?"}\nSearch engines read it.\n:::',
  video:
    '::video{src="/media/site-tour.mp4" title="A tour of the site" poster="/media/site-tour.jpg" description="Where posts, pages and settings live." duration="2:05"}',
};

/** The editor's document for `markdown`, as Tiptap JSON. */
function read(markdown: string) {
  const editor = new Editor({ extensions, content: markdown, contentType: "markdown" });
  const json = editor.getJSON();
  editor.destroy();
  return json;
}

/** The markdown the editor writes for the document `json`. */
function write(json: JSONContent): string {
  const editor = new Editor({ extensions, content: json });
  const markdown = editor.getMarkdown().trimEnd();
  editor.destroy();
  return markdown;
}

describe("the editor's blocks", () => {
  it("has a sample for every directive block", () => {
    for (const spec of BLOCK_SPECS) {
      expect(BLOCKS[spec.name]).toBeDefined();
    }
  });

  for (const [block, markdown] of Object.entries(BLOCKS)) {
    it(`gives back a ${block} unchanged`, () => {
      expect(throughRichText(markdown).trimEnd()).toBe(markdown);
      expect(fitsRichText(markdown)).toBe(true);
    });
  }

  for (const spec of BLOCK_SPECS) {
    it(`reads a ${spec.name} as its own node`, () => {
      const first = read(BLOCKS[spec.name] ?? "").content?.[0];
      expect(first?.type).toBe(`${spec.name}Block`);
    });
  }

  it("reads a block's attributes into the node", () => {
    const button = read(BLOCKS.button ?? "").content?.[0];
    expect(button?.attrs).toMatchObject({ href: "/pricing/", label: "See prices" });
  });

  it("keeps every block in a post in its place", () => {
    const post = Object.values(BLOCKS).join("\n\nA paragraph between blocks.\n\n");
    expect(throughRichText(post).trimEnd()).toBe(post);
  });

  it("writes a changed block in canonical form", () => {
    const doc = read('::button{label="See prices" href="/pricing/"}');
    const button = doc.content?.[0];
    expect(button?.attrs).toBeDefined();
    if (button?.attrs !== undefined) {
      button.attrs.label = "Compare plans";
    }
    expect(write(doc)).toBe('::button{href="/pricing/" label="Compare plans"}');
  });

  it("makes a new block from its attributes alone", () => {
    const doc = {
      type: "doc",
      content: [{ type: "calloutBlock", attrs: { kind: "warning", body: "Back up first." } }],
    };
    expect(write(doc)).toBe(":::callout{kind=warning}\nBack up first.\n:::");
  });

  it("keeps a post with a directive Khalid does not know out of the rich-text view", () => {
    expect(fitsRichText("::tco")).toBe(false);
    expect(fitsRichText(":::note\nHello\n:::")).toBe(false);
  });

  it("keeps a block with an attribute it does not take out of the rich-text view", () => {
    expect(fitsRichText(":::callout{kind=tip colour=blue}\nFeed it daily.\n:::")).toBe(false);
  });
});
