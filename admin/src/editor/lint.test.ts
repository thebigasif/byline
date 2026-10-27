import { describe, expect, it } from "vitest";
import type { Problem } from "../api/posts";
import { lintFindings } from "./lint";

const NO_LINK = {
  rule: "no-internal-link",
  severity: "block",
  locale: "en",
  slug: "planning-a-topic-cluster",
  subject: null,
  what: "This post links to none of your other posts.",
  why: "Internal links are how readers and Google find the rest of your site.",
  fix: "Add one link to a related post.",
  guide: "https://khalid.dev/lint/no-internal-link",
  suggestions: [],
};
const FILLER = { ...NO_LINK, rule: "slug-filler-words", severity: "warn", subject: "a" };

function refused(findings: unknown[]): Problem {
  return {
    type: "https://khalid.dev/problems/lint-blocked",
    title: "The linter stopped this post.",
    status: 422,
    detail: "The linter found something that stops this post from going live.",
    fix: "Fix each finding marked block, then publish again.",
    lint: { blocks: true, findings },
  };
}

describe("lint findings from a refused publish", () => {
  it("reads every finding, blocks before warnings", () => {
    const findings = lintFindings(refused([FILLER, NO_LINK]));
    expect(findings.map((finding) => finding.rule)).toEqual([
      "no-internal-link",
      "slug-filler-words",
    ]);
    expect(findings[1]?.subject).toBe("a");
  });

  it("skips anything that is not a finding", () => {
    expect(lintFindings(refused([{ rule: "x" }, "text", null, NO_LINK]))).toHaveLength(1);
  });

  it("finds nothing in a problem without lint, or no problem", () => {
    expect(lintFindings({ ...refused([]), lint: null })).toEqual([]);
    expect(lintFindings(null)).toEqual([]);
  });
});
