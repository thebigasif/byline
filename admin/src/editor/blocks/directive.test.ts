import { describe, expect, it } from "vitest";
import { printOpening, readAttributes, readOpening, sameValues } from "./directive";

describe("directive attributes", () => {
  it("reads quoted and bare values", () => {
    expect(readAttributes('kind=tip label="See prices"')).toEqual([
      ["kind", "tip"],
      ["label", "See prices"],
    ]);
  });

  it("refuses what khalid-content refuses", () => {
    expect(readAttributes('src="shelf.jpg alt="A shelf"')).toBeNull();
    expect(readAttributes("{kind=tip}")).toBeNull();
    expect(readAttributes("kind")).toBeNull();
    expect(readOpening(":::callout{{kind=tip}}")).toBeNull();
  });

  it("reads an opening line's colons, name and attributes", () => {
    expect(readOpening("::::faq")).toEqual({ fence: "::::", name: "faq", attributes: [] });
    expect(readOpening(":::callout{kind=tip}")?.attributes).toEqual([["kind", "tip"]]);
    expect(readOpening("A paragraph")).toBeNull();
  });

  it("prints attributes in the block's order, quoting all but one word", () => {
    const values = { label: "See prices", href: "/pricing/" };
    expect(printOpening("button", ["href", "label"], values)).toBe(
      'button{href="/pricing/" label="See prices"}',
    );
    expect(printOpening("callout", ["kind"], { kind: "tip" })).toBe("callout{kind=tip}");
    expect(printOpening("toc", [], {})).toBe("toc");
  });

  it("tells when written attributes still match a block's values", () => {
    const written = readAttributes('label="See prices" href="/pricing/"') ?? [];
    const order = ["href", "label"];
    expect(sameValues(written, order, { href: "/pricing/", label: "See prices" })).toBe(true);
    expect(sameValues(written, order, { href: "/pricing/", label: "Compare" })).toBe(false);
    expect(sameValues([["colour", "blue"]], ["kind"], { kind: null })).toBe(false);
  });
});
