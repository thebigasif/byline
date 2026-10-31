import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { adminBlog, noContent } from "../../test/blog";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { AdminComment, CommentStatus } from "../api/comments";
import { en } from "../strings/en";

/** A reader's comment waiting on the internal links post, unless `fields` say otherwise. */
function comment(fields: Partial<AdminComment> = {}): AdminComment {
  return {
    id: 1,
    post: "content/en/posts/2026-09-14-why-internal-links-matter.md",
    author_name: "Jonas",
    author_kind: "reader",
    text: "Do links in the footer count as internal links?",
    status: "pending",
    created_at: "2026-10-30T09:00:00Z",
    ...fields,
  };
}

const PRIYA = comment({ id: 2, author_name: "Priya", text: "This explained our sitemap." });

/** A blog that keeps `kept` and moves them on request. */
function blog(kept: AdminComment[]) {
  let list = [...kept];
  const set = (id: number, status: CommentStatus) => {
    list = list.map((each) => (each.id === id ? { ...each, status } : each));
    return json(200, { comments: list.filter((each) => each.id === id) });
  };
  const sent = adminBlog({
    "GET /comments/": (request) => {
      const status = new URL(request.url).searchParams.get("status");
      return json(200, { comments: list.filter((each) => each.status === status) });
    },
    "POST /comments/1/approve/": () => set(1, "approved"),
    "DELETE /comments/2/": () => {
      list = list.filter((each) => each.id !== 2);
      return noContent();
    },
  });
  return { sent };
}

/** The list item that holds the comment saying `text`. */
async function cardOf(text: string): Promise<HTMLElement> {
  const card = (await screen.findByText(text)).closest("li");
  if (card === null) {
    throw new Error(`No list item holds "${text}"`);
  }
  return card;
}

describe("the Comments screen", () => {
  it("lists the waiting comments with the next one first", async () => {
    signedIn();
    const { sent } = blog([comment(), PRIYA]);
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.comments }));
    expect(await screen.findByRole("heading", { level: 1, name: en.comments.title })).toBeVisible();
    const jonas = await cardOf(comment().text);
    expect(jonas).toHaveTextContent("why-internal-links-matter");
    expect(jonas).toHaveClass("focus");
    expect(await cardOf(PRIYA.text)).toHaveClass("card");
    expect(screen.getByRole("button", { name: en.comments.statuses.pending })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(sent.some((request) => request.url.includes("/comments/?status=pending"))).toBe(true);
    expect(await axeViolations()).toEqual([]);
  });

  it("approves a comment, and it leaves the waiting list", async () => {
    signedIn();
    blog([comment(), PRIYA]);
    renderAt("/comments");
    await userEvent.click(
      await screen.findByRole("button", { name: en.comments.approveLabel("Jonas") }),
    );
    expect(await screen.findByText(en.comments.approved("Jonas"))).toBeVisible();
    await waitFor(() => expect(screen.queryByText(comment().text)).toBeNull());
    await userEvent.click(screen.getByRole("button", { name: en.comments.statuses.approved }));
    expect(await screen.findByText(comment().text)).toBeVisible();
  });

  it("deletes a comment", async () => {
    signedIn();
    const { sent } = blog([comment(), PRIYA]);
    renderAt("/comments");
    await userEvent.click(
      await screen.findByRole("button", { name: en.comments.removeLabel("Priya") }),
    );
    expect(await screen.findByText(en.comments.removed("Priya"))).toBeVisible();
    expect(sent.filter((request) => request.method === "DELETE")).toHaveLength(1);
  });

  it("says when nothing waits, and shows the API's words when it refuses", async () => {
    signedIn();
    blog([]);
    renderAt("/comments");
    expect(await screen.findByText(en.comments.none.pending)).toBeVisible();

    const lacks = {
      type: "https://khalid.dev/problems/forbidden",
      title: "This key cannot moderate comments.",
      status: 403,
      detail: "Comments need the posts:write scope.",
      fix: "Sign in with an editor's staff token.",
    };
    adminBlog({ "GET /comments/": () => json(403, lacks) });
    await userEvent.click(screen.getByRole("button", { name: en.comments.statuses.spam }));
    expect(await screen.findByRole("alert")).toHaveTextContent(lacks.title);
  });
});
