import { screen, waitFor, within } from "@testing-library/react";
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
    return rowsPage(rows, total, Number(query.get("page") ?? "1"), 20);
  });
  return queries;
}

describe("filtering and paging posts", () => {
  it("asks the blog only for posts the address filters to", async () => {
    signedIn();
    const queries = blog([ROWS[1] ?? row()]);
    renderAt("/posts?checks=blocked&stage=in_review");
    await screen.findByRole("table");
    expect(queries[0]?.get("filter")).toBe("gate:refuses+status:draft+review_state:in_review");
    expect(screen.getByLabelText(en.posts.checks)).toHaveValue("blocked");
  });

  it("writes the picked filters into the address and lists again from page 1", async () => {
    signedIn();
    const queries = blog();
    const { router } = renderAt("/posts?page=2");
    const form = await screen.findByRole("form", { name: en.filters.label });
    await userEvent.selectOptions(within(form).getByLabelText(en.posts.index), "noindex");
    await userEvent.click(within(form).getByRole("button", { name: en.filters.show }));
    await waitFor(() => expect(queries.at(-1)?.get("filter")).toBe("index_status:noindex"));
    expect(queries.at(-1)?.get("page")).toBe("1");
    expect(router.state.location.search).toEqual({ index: "noindex" });
    expect(await axeViolations()).toEqual([]);
    router.history.back();
    await waitFor(() => expect(screen.getByLabelText(en.posts.index)).toHaveValue(""));
  });

  it("moves between pages and keeps the filters", async () => {
    signedIn();
    const queries = blog(ROWS, 45);
    renderAt("/posts?stage=live");
    const pages = await screen.findByRole("navigation", { name: en.paging.label });
    expect(pages).toHaveTextContent(en.paging.where(1, 3));
    expect(within(pages).queryByRole("link", { name: en.paging.previous })).toBeNull();
    await userEvent.click(within(pages).getByRole("link", { name: en.paging.next }));
    await screen.findByText(en.paging.where(2, 3));
    expect(queries.at(-1)?.get("page")).toBe("2");
    expect(queries.at(-1)?.get("filter")).toBe("status:published");
  });
});

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
    expect(router.state.location.pathname).toBe("/new");
    expect(await axeViolations()).toEqual([]);
  });
});
