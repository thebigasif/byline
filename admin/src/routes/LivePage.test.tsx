import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import type { AdminPost } from "../api/posts";
import { onThisBlog } from "../editor/LivePage";
import { en } from "../strings/en";

const DRAFT: AdminPost = {
  id: "01JB",
  slug: "planning-a-topic-cluster",
  title: "Planning a topic cluster",
  status: "draft",
  markdown: "A topic cluster is a group of posts.\n",
  updated_at: "2026-10-27T10:42:00Z",
  settings: {
    co_authors: [],
    featured: false,
    review_state: "draft",
    access: "public",
    password_protected: false,
    share_cards: {},
    aliases: [],
  },
};

const LINK = {
  url: "https://www.example.com/blog/preview/en.planning/?until=1793696400&sig=c2ln",
  expires_at: "2026-11-03T10:42:00Z",
};

/** A blog holding DRAFT, with preview links answered by `preview`. */
function blog(preview = 201) {
  let saves = 0;
  serve(async (request) => {
    if (request.url.endsWith("/preview-link/")) {
      return preview === 201
        ? json(201, { preview_links: [LINK] })
        : json(preview, {
            title: "This site cannot make preview links yet.",
            detail: "Preview links need a signing key, and this site has none.",
            fix: "Run khalid serve once more; it makes the key on start.",
          });
    }
    if (request.method !== "PUT") {
      return json(200, { posts: [DRAFT] });
    }
    saves += 1;
    return json(200, { posts: [{ ...DRAFT, updated_at: `2026-10-27T10:4${saves}:30Z` }] });
  });
}

const showPage = async () => {
  renderAt("/posts/01JB");
  fireEvent.click(await screen.findByRole("button", { name: en.live.show }));
};

describe("the page beside the text", () => {
  it("reads a preview link's address on this blog", () => {
    expect(onThisBlog(LINK.url)).toBe("/blog/preview/en.planning/?until=1793696400&sig=c2ln");
  });

  it("shows the saved draft in the site's design, in place of the side column", async () => {
    signedIn();
    blog();
    await showPage();
    expect(screen.getByRole("button", { name: en.live.show })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    const page = await screen.findByTitle(en.live.frame);
    expect(page).toHaveAttribute("src", onThisBlog(LINK.url));
    expect(screen.queryByRole("button", { name: en.preview.make })).not.toBeInTheDocument();
    expect(await axeViolations()).toEqual([]);
  });

  it("loads the page again after each save", async () => {
    signedIn();
    blog();
    await showPage();
    const before = await screen.findByTitle(en.live.frame);
    const title = screen.getByLabelText(en.editor.title);
    fireEvent.change(title, { target: { value: "Planning a topic cluster in an hour" } });
    fireEvent.blur(title);
    await waitFor(() => expect(screen.getByTitle(en.live.frame)).not.toBe(before));
    expect(screen.getByRole("status")).toHaveTextContent(en.editor.saved);
  });

  it("says why when the blog cannot make a preview link", async () => {
    signedIn();
    blog(409);
    await showPage();
    const problem = await screen.findByRole("alert");
    expect(problem).toHaveTextContent("This site cannot make preview links yet.");
    expect(screen.queryByTitle(en.live.frame)).not.toBeInTheDocument();
  });
});
