import { describe, expect, it } from "vitest";
import { goLiveEdit, localInput } from "./schedule";

describe("going live", () => {
  it("publishes now when no time is picked", () => {
    expect(goLiveEdit("2026-10-27T10:42:00Z", "")).toEqual({
      updated_at: "2026-10-27T10:42:00Z",
      status: "published",
    });
  });

  it("refuses a time that is not one", () => {
    expect(goLiveEdit("2026-10-27T10:42:00Z", "2026-11-03T")).toBeNull();
  });

  it("reads a moment back into the field as the writer's local time", () => {
    const at = "2026-11-03T09:05";
    expect(localInput(new Date(at).toISOString())).toBe(at);
  });
});
