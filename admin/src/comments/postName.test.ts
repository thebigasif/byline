import { describe, expect, it } from "vitest";
import { postName } from "./postName";

describe("a post's name from its file", () => {
  it("drops the folder, the date and the extension", () => {
    expect(postName("content/en/posts/2026-09-14-why-internal-links-matter.md")).toBe(
      "why-internal-links-matter",
    );
  });

  it("names a post kept as a folder by the folder", () => {
    expect(postName("content/en/posts/2026-10-05-fixing-slow-pages/index.md")).toBe(
      "fixing-slow-pages",
    );
  });

  it("keeps a file with no date as it is", () => {
    expect(postName("content/de/posts/sitemaps.md")).toBe("sitemaps");
  });
});
