import { describe, expect, it } from "vitest";
import { lineDiff } from "./diff";

describe("comparing two texts line by line", () => {
  it("marks a changed line as removed then added, and keeps the rest", () => {
    const before = "## Why links matter\nInternal links help Google.\nStart with one.";
    const after = "## Why links matter\nInternal links help readers and Google.\nStart with one.";
    expect(lineDiff(before, after)).toEqual([
      { kind: "same", text: "## Why links matter" },
      { kind: "removed", text: "Internal links help Google." },
      { kind: "added", text: "Internal links help readers and Google." },
      { kind: "same", text: "Start with one." },
    ]);
  });

  it("shows lines added at the end and removed at the start", () => {
    expect(lineDiff("Old intro.\nKept.", "Kept.\nNew close.")).toEqual([
      { kind: "removed", text: "Old intro." },
      { kind: "same", text: "Kept." },
      { kind: "added", text: "New close." },
    ]);
  });

  it("finds nothing to mark in the same text", () => {
    const text = "One.\n\nTwo.";
    expect(lineDiff(text, text).every((line) => line.kind === "same")).toBe(true);
  });
});
