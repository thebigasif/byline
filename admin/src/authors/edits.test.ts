import { describe, expect, it } from "vitest";
import type { AdminAuthor } from "../api/authors";
import { changes, fieldsOf } from "./edits";

const ASIF: AdminAuthor = {
  id: "en.asif",
  slug: "asif",
  language: "en",
  name: "Asif",
  role: "Founder of SerpDad",
  bio: "Asif writes about search.\n",
  same_as: ["https://serpdad.com/"],
  updated_at: "2026-10-30T09:00:00Z",
};

describe("an author's edit", () => {
  it("is nothing when no field changed", () => {
    expect(changes(ASIF, fieldsOf(ASIF))).toBeNull();
  });

  it("holds only the changed fields, with the copy's time", () => {
    const fields = {
      ...fieldsOf(ASIF),
      role: "",
      sameAs: "https://serpdad.com/\n\n https://x.com/asif ",
    };
    expect(changes(ASIF, fields)).toEqual({
      updated_at: ASIF.updated_at,
      role: "",
      same_as: ["https://serpdad.com/", "https://x.com/asif"],
    });
  });
});
