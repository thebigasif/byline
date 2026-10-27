import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import { listing, row, rowsPage } from "../../test/rows";
import type { PostRow } from "../api/rows";
import { en } from "../strings/en";

const ROWS = [
  row({ lint: { blocks: 0, warnings: 3 } }),
  row({
    id: "01JB",
    title: "Planning a topic cluster in an afternoon",
    author: null,
    status: "draft",
    review_state: "in_review",
    gate: "refuses",
    lint: { blocks: 1, warnings: 0 },
    index_status: "not_live",
  }),
];

/** A blog that lists `rows` as one page of `total`, and the list queries it was sent. */
function blog(rows: PostRow[] = ROWS, total = rows.length) {
  const queries: URLSearchParams[] = [];
  serve((request) => {
    const query = listing(request);
    if (query === null) {
      return json(404, {});
    }
    queries.push(query);
    return rowsPage(rows, total);
  });
  return queries;
}

describe("the posts screen", () => {
  it("lists every post, newest change first, with what each stands at", async () => {
    signedIn();
    const queries = blog();
    renderAt("/posts");
    const table = await screen.findByRole("table");
    const [, live, stuck] = within(table).getAllByRole("row");
    expect(live).toHaveTextContent(en.posts.warnings(3));
    expect(live).toHaveTextContent(en.stage.live);
    expect(live).toHaveTextContent(en.index.indexable);
    expect(live).toHaveTextContent("asif");
    expect(stuck).toHaveTextContent(en.posts.blocks(1));
    expect(stuck).toHaveTextContent(en.stage.in_review);
    expect(stuck).toHaveTextContent(en.posts.noAuthor);
    const link = within(table).getByRole("link", { name: ROWS[1]?.title ?? "" });
    expect(link).toHaveAttribute("href", "/admin/posts/01JB");
    expect(screen.getByRole("status")).toHaveTextContent(en.posts.found(2));
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(queries[0]?.get("order")).toBe("updated_at desc");
    expect(queries[0]?.get("filter")).toBeNull();
    expect(await axeViolations()).toEqual([]);
  });

  it("says when no post matches", async () => {
    signedIn();
    blog([]);
    renderAt("/posts");
    expect(await screen.findByText(en.posts.none)).toHaveAttribute("role", "status");
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("explains a list the blog would not send", async () => {
    signedIn();
    serve(() => {
      throw new TypeError("Failed to fetch");
    });
    renderAt("/posts");
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.unreachable.what);
  });

  it("opens the start form from Write a new post", async () => {
    signedIn();
    blog();
    const { router } = renderAt("/posts");
    await userEvent.click(await screen.findByRole("link", { name: en.posts.write }));
    expect(await screen.findByRole("heading", { level: 1, name: en.posts.write })).toBeVisible();
    expect(screen.getByLabelText(en.start.label)).toBeVisible();
    expect(router.state.location.pathname).toBe("/admin/new");
    expect(await axeViolations()).toEqual([]);
  });
});
