import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { redirectsBlog } from "../../test/redirects";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import { row, rowsPage } from "../../test/rows";
import type { AdminPost, PostEdit } from "../api/posts";
import { en } from "../strings/en";

const POST: AdminPost = {
  id: "01JB",
  slug: "planning-a-topic-cluster",
  title: "Planning a topic cluster",
  status: "published",
  author: "asif",
  markdown: "A topic cluster is a group of posts.\n",
  updated_at: "2026-10-27T10:42:00Z",
  settings: {
    co_authors: [],
    featured: false,
    review_state: "ready",
    access: "public",
    password_protected: false,
    share_cards: {},
    aliases: ["/blog/clusters-2019/"],
  },
};

/** A blog with two live posts, which records every edit and answers it with `answer`. */
function blog(answer = () => json(200, { posts: [POST] })) {
  const edits: PostEdit[] = [];
  redirectsBlog({
    "GET /posts/": () => rowsPage([row(), row({ id: POST.id, title: POST.title })]),
    "GET /posts/01JB/": () => json(200, { posts: [POST] }),
    "PUT /posts/01JB/": async (request) => {
      const body: { posts: PostEdit[] } = await request.json();
      edits.push(...body.posts);
      return answer();
    },
  });
  return edits;
}

/** Fills in the form with `from` and the post titled `title`, and sends it. */
async function send(from: string, title = POST.title) {
  await userEvent.type(await screen.findByLabelText(en.redirects.send.from), from);
  await userEvent.selectOptions(screen.getByLabelText(en.redirects.send.to), title);
  await userEvent.click(screen.getByRole("button", { name: en.redirects.send.submit }));
}

describe("sending an old address to a post", () => {
  it("adds the address to the post's old addresses, and says where to find it", async () => {
    signedIn();
    const edits = blog();
    renderAt("/redirects");
    await send("/blog/clusters/");
    const added = en.redirects.send.done("/blog/clusters/");
    const done = await screen.findByText(added, { exact: false });
    expect(done).toHaveTextContent(en.redirects.send.where);
    const link = screen.getByRole("link", { name: POST.title });
    expect(link).toHaveAttribute("href", "/admin/posts/01JB");
    expect(edits).toEqual([
      {
        updated_at: POST.updated_at,
        settings: { aliases: ["/blog/clusters-2019/", "/blog/clusters/"] },
      },
    ]);
    expect(await axeViolations()).toEqual([]);
  });

  it("explains an address that is not a path, and saves nothing", async () => {
    signedIn();
    const edits = blog();
    renderAt("/redirects");
    await send("blog/clusters");
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.badAlias.what);
    expect(edits).toEqual([]);
  });

  it("shows the API's words when the post changed under the form", async () => {
    signedIn();
    const stale = {
      type: "https://khalid.dev/problems/conflict",
      title: "Someone else changed this post first.",
      status: 409,
      detail: "Saving now would undo their change.",
      fix: "Send the old address again.",
    };
    blog(() => json(409, stale));
    renderAt("/redirects");
    await send("/blog/clusters/");
    expect(await screen.findByRole("alert")).toHaveTextContent(stale.title);
  });
});
