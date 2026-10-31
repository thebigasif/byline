import { describe, expect, it } from "vitest";
import { aiVisitors, analyticsBlog, uncited, visitors } from "../../test/analytics";
import { json, serve, serverAnswers, signedIn } from "../../test/render";
import { readAiVisitors, readUncited, readVisitors } from "./analytics";

const LACKS = {
  type: "https://khalid.dev/problems/forbidden",
  title: "This key cannot read visitor data.",
  status: 403,
  detail: "Analytics need the settings:write scope.",
  fix: "Sign in with an owner or admin staff token.",
};

describe("the analytics client", () => {
  it("reads each report for the days asked for, with the signed-in key", async () => {
    signedIn();
    const sent = analyticsBlog();
    expect(await readVisitors(7)).toEqual({ data: visitors() });
    expect(await readAiVisitors(28)).toEqual({ data: aiVisitors() });
    expect(await readUncited(90)).toEqual({ data: uncited() });
    const asked = sent.map((request) => {
      const url = new URL(request.url);
      return `${url.pathname}${url.search}`;
    });
    expect(asked).toEqual([
      "/api/admin/v1/analytics/visitors/?days=7",
      "/api/admin/v1/analytics/ai-visitors/?days=28",
      "/api/admin/v1/analytics/uncited/?days=90",
    ]);
    expect(sent[0]?.headers.get("Authorization")).toBe("Khalid ks_0123456789abcdef_test");
  });

  it("gives back the API's problem when the key may not read visitor data", async () => {
    analyticsBlog({ "GET /analytics/visitors/": () => json(403, LACKS) });
    expect(await readVisitors(28)).toEqual({ problem: LACKS, status: 403 });
  });

  it("keeps the status when the blog sends no problem", async () => {
    serverAnswers(502);
    expect(await readUncited(28)).toEqual({ problem: null, status: 502 });
  });

  it("answers status 0 when the blog cannot be reached", async () => {
    serve(() => {
      throw new TypeError("Failed to fetch");
    });
    expect(await readAiVisitors(7)).toEqual({ problem: null, status: 0 });
  });
});
