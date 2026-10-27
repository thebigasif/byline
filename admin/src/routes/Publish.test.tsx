import { fireEvent, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import type { AdminPost, PostEdit } from "../api/posts";
import { en } from "../strings/en";

const DRAFT: AdminPost = {
  id: "01JB",
  slug: "planning-a-topic-cluster",
  title: "Planning a topic cluster",
  status: "draft",
  markdown: ':::callout{tone="tip"}\nPlan on one page.\n:::\n',
  updated_at: "2026-10-27T10:42:00Z",
  settings: {
    co_authors: [],
    featured: false,
    review_state: "draft",
    share_cards: {},
    aliases: [],
  },
};

const REFUSED = {
  type: "https://khalid.dev/problems/lint-blocked",
  title: "The linter stopped this post.",
  status: 422,
  detail: "The linter found something that stops this post from going live.",
  fix: "Fix each finding marked block, then publish again.",
  lint: {
    blocks: true,
    findings: [
      {
        rule: "no-internal-link",
        severity: "block",
        subject: null,
        what: "This post links to none of your other posts.",
        why: "Internal links are how readers and Google find the rest of your site.",
        fix: "Add one link to a related post.",
      },
    ],
  },
};

/** A blog that answers each edit with the next answer in `answers`. */
function blog(...answers: ((edit: PostEdit) => Response)[]) {
  const edits: PostEdit[] = [];
  serve(async (request) => {
    if (request.method !== "PUT") {
      return json(200, { posts: [DRAFT] });
    }
    const body: { posts: PostEdit[] } = await request.json();
    const edit = body.posts[0] ?? { updated_at: "" };
    edits.push(edit);
    const next = answers[edits.length - 1];
    return next === undefined ? json(500, {}) : next(edit);
  });
  return edits;
}

const saved = (at: string) => () => json(200, { posts: [{ ...DRAFT, updated_at: at }] });

describe("publishing from the editor", () => {
  it("shows every finding when the linter stops the post", async () => {
    const edits = blog(() => json(422, REFUSED));
    signedIn();
    renderAt("/posts/01JB");
    fireEvent.click(await screen.findByRole("button", { name: en.editor.publish }));
    const panel = await screen.findByRole("complementary", { name: en.editor.checks });
    const block = await within(panel).findByRole("region", { name: en.editor.blockTag });
    expect(block).toHaveTextContent("This post links to none of your other posts.");
    expect(block).toHaveTextContent("Add one link to a related post.");
    expect(screen.getByText(en.editor.blocked)).toBeVisible();
    expect(edits).toEqual([{ updated_at: "2026-10-27T10:42:00Z", status: "published" }]);
    expect(await axeViolations()).toEqual([]);
  });

  it("saves the latest words first, then publishes that copy", async () => {
    const edits = blog(saved("2026-10-27T10:50:00Z"), () =>
      json(200, { posts: [{ ...DRAFT, status: "published" }] }),
    );
    signedIn();
    renderAt("/posts/01JB");
    const text = await screen.findByRole("textbox", { name: en.editor.text });
    fireEvent.change(text, { target: { value: "Plan on one page.\n" } });
    fireEvent.click(screen.getByRole("button", { name: en.editor.publish }));
    expect(await screen.findByText(en.editor.published)).toBeVisible();
    expect(edits[0]?.markdown).toBe("Plan on one page.\n");
    expect(edits[1]).toEqual({ updated_at: "2026-10-27T10:50:00Z", status: "published" });
  });

  it("explains a publish the blog did not answer", async () => {
    blog();
    signedIn();
    renderAt("/posts/01JB");
    fireEvent.click(await screen.findByRole("button", { name: en.editor.publish }));
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.unreachable.what);
  });
});
