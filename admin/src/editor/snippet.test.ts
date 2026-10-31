import { describe, expect, it } from "vitest";
import { shownAddress } from "./snippet";

describe("the address in a search result", () => {
  it("shows the site, then each folder", () => {
    const url = "https://www.example.com/blog/why-internal-links-matter/";
    expect(shownAddress(url)).toBe("https://www.example.com › blog › why-internal-links-matter");
  });

  it("shows anything that is not an address as it is", () => {
    expect(shownAddress("")).toBe("");
  });
});
