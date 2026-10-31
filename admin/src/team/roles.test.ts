import { describe, expect, it } from "vitest";
import { en } from "../strings/en";
import { GIVEN, roleName } from "./roles";

describe("team roles in words", () => {
  it("offers every role but owner, each explained", () => {
    expect(GIVEN).toEqual(["writer", "contributor", "editor", "admin"]);
    for (const role of GIVEN) {
      expect(en.team.roleHints[role]).not.toBe("");
    }
  });

  it("names a role, and an unknown one by its API name", () => {
    expect(roleName("owner")).toBe("Owner");
    expect(roleName("editor")).toBe("Editor");
    expect(roleName("reviewer")).toBe("reviewer");
  });
});
