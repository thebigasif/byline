import { describe, expect, it } from "vitest";
import { adminBlog, noContent } from "../../test/blog";
import { json, signedIn } from "../../test/render";
import { isData } from "./call";
import { approveComment, deleteComment, listComments, replyToComment } from "./comments";

const GONE = {
  type: "https://khalid.dev/problems/not-found",
  title: "No comment has this number.",
  status: 404,
  detail: "It may have been deleted already.",
  fix: "Reload the list of comments.",
};

describe("the comment routes", () => {
  it("asks for one status, with the signed-in key", async () => {
    signedIn();
    const sent = adminBlog({ "GET /comments/": () => json(200, { comments: [] }) });
    expect(await listComments("spam")).toEqual({ data: [] });
    expect(sent[0]?.url).toMatch(/\/api\/admin\/v1\/comments\/\?status=spam&limit=100$/);
    expect(sent[0]?.headers.get("Authorization")).toMatch(/^Khalid /);
  });

  it("sends a reply as the one comment the API expects", async () => {
    signedIn();
    const sent = adminBlog({ "POST /comments/7/replies/": () => json(201, { comments: [] }) });
    await replyToComment(7, "Asif", "Thanks.");
    expect(await sent[0]?.json()).toEqual({ comments: [{ author_name: "Asif", text: "Thanks." }] });
  });

  it("deletes with no body to read, and gives back the API's problem", async () => {
    signedIn();
    adminBlog({
      "DELETE /comments/7/": noContent,
      "POST /comments/8/approve/": () => json(404, GONE),
    });
    expect(isData(await deleteComment(7))).toBe(true);
    expect(await approveComment(8)).toEqual({ problem: GONE, status: 404 });
  });

  it("says the blog is unreachable when nothing answers", async () => {
    signedIn();
    adminBlog({ "GET /comments/": () => Promise.reject(new TypeError("offline")) });
    expect(await listComments("pending")).toEqual({ problem: null, status: 0 });
  });
});
