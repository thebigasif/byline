import type { ChainedCommands } from "@tiptap/core";
import type { Sources, StockSource } from "../../api/pictures";
import { en } from "../../strings/en";
import { pictures } from "../../strings/pictures";
import { tables } from "../../strings/tables";
import { nodeName } from "../blocks/directiveNode";
import { BLOCK_SPECS, type BlockSpec } from "../blocks/specs";
import { emptyGrid, writeTable } from "../blocks/table";

/** One line of the slash menu: a block the writer can add. */
export type SlashItem = {
  id: string;
  name: string;
  hint: string;
  /** The directive this item adds, so the details form can ask for its attributes. */
  spec: BlockSpec | null;
  /** Turns the writer's empty line into this block. */
  add: (chain: ChainedCommands) => ChainedCommands;
  /** The search this item opens instead, offered only when the owner turned it on. */
  source?: StockSource;
};

/** An item that opens photo or GIF search; the picture is added once picked. */
const search = (source: StockSource, name: string, hint: string): SlashItem => ({
  id: source,
  name,
  hint,
  spec: null,
  add: (chain) => chain,
  source,
});

const starter = (
  id: string,
  name: string,
  hint: string,
  add: (chain: ChainedCommands) => ChainedCommands,
): SlashItem => ({ id, name, hint, spec: null, add });

/** The table item, which opens an empty grid rather than adding a block at once. */
export const TABLE_ITEM = "table";

const names: Record<string, string> = en.blocks;
const hints: Record<string, string> = en.blockHints;

/** The block a directive item adds, with no details yet. */
function directive(spec: BlockSpec): SlashItem {
  return {
    id: spec.name,
    name: names[spec.name] ?? spec.name,
    hint: hints[spec.name] ?? "",
    spec,
    add: (chain) => chain.insertContent({ type: nodeName(spec), attrs: {} }),
  };
}

/** Every block the slash menu offers: the starter kit's first, then each directive. */
export const SLASH_ITEMS: readonly SlashItem[] = [
  starter("heading", en.aids.heading, en.aids.headingHint, (chain) =>
    chain.setNode("heading", { level: 2 }),
  ),
  starter("subheading", en.aids.subheading, en.aids.subheadingHint, (chain) =>
    chain.setNode("heading", { level: 3 }),
  ),
  starter("bullets", en.aids.bullets, en.aids.bulletsHint, (chain) => chain.toggleBulletList()),
  starter("numbers", en.aids.numbers, en.aids.numbersHint, (chain) => chain.toggleOrderedList()),
  starter("quote", en.aids.quote, en.aids.quoteHint, (chain) => chain.toggleBlockquote()),
  starter("code", en.aids.code, en.aids.codeHint, (chain) => chain.toggleCodeBlock()),
  starter("divider", en.aids.divider, en.aids.dividerHint, (chain) => chain.setHorizontalRule()),
  starter(TABLE_ITEM, tables.title, tables.slashHint, (chain) =>
    chain.insertContent({ type: "markdownTable", attrs: { source: writeTable(emptyGrid()) } }),
  ),
  ...BLOCK_SPECS.map(directive),
  search("unsplash", pictures.unsplash, pictures.unsplashSlashHint),
  search("gifs", pictures.gifs, pictures.gifsSlashHint),
];

/** How many lines the menu shows at once; typing narrows the rest. */
export const MENU_LENGTH = 8;

/**
 * The items whose name or directive starts a word with `query`, names that
 * start with it first, then in menu order. A search item shows only when
 * `sources` says the owner turned it on.
 */
export function matchingItems(query: string, sources?: Sources): SlashItem[] {
  const wanted = query.toLowerCase();
  const scored = SLASH_ITEMS.flatMap((item, index) => {
    if (item.source !== undefined && sources?.[item.source] !== true) {
      return [];
    }
    const name = item.name.toLowerCase();
    if (name.startsWith(wanted) || item.id.startsWith(wanted)) {
      return [{ item, rank: index }];
    }
    const words = `${name} ${item.hint.toLowerCase()}`.split(/[^\p{L}\p{N}]+/u);
    return words.some((word) => word.startsWith(wanted))
      ? [{ item, rank: SLASH_ITEMS.length + index }]
      : [];
  });
  return scored
    .sort((a, b) => a.rank - b.rank)
    .slice(0, MENU_LENGTH)
    .map((found) => found.item);
}
