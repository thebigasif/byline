import { describe, expect, it } from "vitest";
import { json, serve, serverAnswers, signedIn } from "../../test/render";
import { followUps, NOT_SET_UP, searchConsoleBlog } from "../../test/searchConsole";
import { openSearches, readFollowUps } from "./searchConsole";

describe("the Search Console client", () => {
  it("reads the follow-ups with the signed-in key", async () => {
    signedIn();
    const sent = searchConsoleBlog();
    expect(await readFollowUps()).toEqual({ data: followUps() });
    expect(sent.map((request) => new URL(request.url).pathname)).toEqual([
      "/api/admin/v1/search-console/follow-ups/",
    ]);
    expect(sent[0]?.headers.get("Authorization")).toBe("Khalid ks_0123456789abcdef_test");
  });

  it("gives back the API's problem when the blog is not connected", async () => {
    searchConsoleBlog({ "GET /search-console/follow-ups/": () => json(409, NOT_SET_UP) });
    expect(await readFollowUps()).toEqual({ problem: NOT_SET_UP, status: 409 });
  });

  it("keeps the status when the blog sends no problem", async () => {
    serverAnswers(502);
    expect(await readFollowUps()).toEqual({ problem: null, status: 502 });
  });

  it("answers status 0 when the blog cannot be reached", async () => {
    serve(() => {
      throw new TypeError("Failed to fetch");
    });
    expect(await readFollowUps()).toEqual({ problem: null, status: 0 });
  });

  it("counts the searches no heading answers yet", () => {
    expect(openSearches(followUps())).toEqual({ open: 2, all: 3 });
    expect(openSearches(followUps({ posts: [] }))).toEqual({ open: 0, all: 0 });
  });
});
