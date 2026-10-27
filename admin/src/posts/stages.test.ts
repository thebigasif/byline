import { describe, expect, it } from "vitest";
import { row } from "../../test/rows";
import { stageOf } from "./stages";

describe("a post's stage", () => {
  it("comes from its status, or its review state while a draft", () => {
    expect(stageOf(row({ status: "published" }))).toBe("live");
    expect(stageOf(row({ status: "scheduled" }))).toBe("scheduled");
    expect(stageOf(row({ status: "draft", review_state: "in_review" }))).toBe("in_review");
  });
});
