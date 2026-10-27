import { describe, expect, it } from "vitest";
import { json, serve } from "../../test/render";
import { row, rowsPage } from "../../test/rows";
import { listPosts } from "./rows";

/** A blog that answers every list with `answer`, and the requests it was sent. */
function blog(answer: () => Response) {
  const sent: Request[] = [];
  serve((request) => {
    sent.push(request);
    return answer();
  });
  return sent;
}

describe("listing posts", () => {
  it("sends the filter, order and page in the API's grammar", async () => {
    const sent = blog(() => rowsPage([row()], 31, 3));
    const answer = await listPosts({
      filter: "gate:refuses+status:draft",
      order: "updated_at desc",
      limit: 15,
      page: 3,
    });
    const url = new URL(sent[0]?.url ?? "http://localhost/");
    expect(url.pathname).toBe("/api/admin/v1/posts/");
    // A bare + would reach the server as a space and change the filter.
    expect(url.search).toContain("%2B");
    expect(url.searchParams.get("filter")).toBe("gate:refuses+status:draft");
    expect(url.searchParams.get("order")).toBe("updated_at desc");
    expect(url.searchParams.get("limit")).toBe("15");
    expect(answer).toEqual({
      rows: [row()],
      pagination: { page: 3, limit: 15, pages: 3, total: 31, next: null, prev: 2 },
    });
  });

  it("leaves out what the query does not set", async () => {
    const sent = blog(() => rowsPage([]));
    await listPosts({});
    expect(new URL(sent[0]?.url ?? "http://localhost/?x").search).toBe("");
  });

  it("hands back the problem the API sent", async () => {
    const problem = {
      type: "https://khalid.dev/problems/bad-list",
      title: "This list cannot be read.",
      status: 400,
      detail: "Posts have no field called colour.",
      fix: "Filter by a field a post has, such as status.",
    };
    blog(() => json(400, problem));
    expect(await listPosts({ filter: "colour:red" })).toEqual({ problem, status: 400 });
  });

  it("answers status 0 when the blog cannot be reached", async () => {
    serve(() => {
      throw new TypeError("Failed to fetch");
    });
    expect(await listPosts({})).toEqual({ problem: null, status: 0 });
  });
});
