import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import type { AdminPost } from "../api/posts";
import { en } from "../strings/en";

const CALLOUT = ':::callout{tone="tip"}\nFeed the starter daily.\n:::\n';

function post(markdown: string): AdminPost {
  return {
    id: "01JB",
    slug: "planning-a-topic-cluster",
    title: "Planning a topic cluster",
    status: "draft",
    markdown,
    updated_at: "2026-10-27T10:42:00Z",
    settings: {
      co_authors: [],
      featured: false,
      review_state: "draft",
      share_cards: {},
      aliases: [],
    },
  };
}

/** A blog holding one post, which records every edit it is sent. */
function blog(markdown: string, editStatus = 200) {
  const edits: unknown[] = [];
  serve(async (request) => {
    if (request.method !== "PUT") {
      return json(200, { posts: [post(markdown)] });
    }
    const body: { posts: AdminPost[] } = await request.json();
    edits.push(body.posts[0]);
    if (editStatus === 200) {
      return json(200, { posts: [{ ...post(markdown), updated_at: "2026-10-27T10:44:00Z" }] });
    }
    return json(editStatus, {
      type: "https://khalid.dev/problems/stale",
      title: "Someone else changed this post.",
      status: editStatus,
      detail: "Lena Vogt saved it at 10:43, after you opened it.",
      fix: "Copy your text, reload the post, and add your change again.",
    });
  });
  return edits;
}

describe("writing in the editor", () => {
  it("opens a plain post as rich text", async () => {
    signedIn();
    blog("A topic cluster is a group of posts.\n");
    renderAt("/posts/01JB");
    const rich = await screen.findByRole("button", { name: en.editor.rich });
    expect(rich).toHaveAttribute("aria-pressed", "true");
    expect(await axeViolations()).toEqual([]);
  });

  it("keeps a post with a directive in markdown and saves it on blur", async () => {
    signedIn();
    const edits = blog(CALLOUT);
    renderAt("/posts/01JB");
    const text = await screen.findByRole("textbox", { name: en.editor.text });
    expect(screen.getByText(en.editor.onlyMarkdown)).toBeVisible();
    fireEvent.change(text, { target: { value: `${CALLOUT}\nOne more line.\n` } });
    fireEvent.blur(text);
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent(en.editor.saved));
    expect(edits).toEqual([
      {
        updated_at: "2026-10-27T10:42:00Z",
        title: "Planning a topic cluster",
        markdown: `${CALLOUT}\nOne more line.\n`,
      },
    ]);
  });

  it("shows who changed the post and stops saving on a 409", async () => {
    signedIn();
    const edits = blog(CALLOUT, 409);
    renderAt("/posts/01JB");
    const text = await screen.findByRole("textbox", { name: en.editor.text });
    fireEvent.change(text, { target: { value: "Mine" } });
    fireEvent.blur(text);
    expect(await screen.findByRole("alert")).toHaveTextContent("Lena Vogt saved it at 10:43");
    fireEvent.change(text, { target: { value: "Mine, again" } });
    fireEvent.blur(text);
    expect(edits).toHaveLength(1);
    expect(screen.getByRole("status")).toHaveTextContent(en.editor.unsaved);
  });

  it("asks before leaving with changes not saved yet", async () => {
    signedIn();
    blog(CALLOUT);
    const { router } = renderAt("/posts/01JB");
    const text = await screen.findByRole("textbox", { name: en.editor.text });
    fireEvent.change(text, { target: { value: "Not saved yet" } });
    fireEvent.click(screen.getByRole("link", { name: en.nav.home }));
    expect(await screen.findByRole("alertdialog")).toHaveTextContent(en.editor.leave);
    fireEvent.click(screen.getByRole("button", { name: en.editor.stay }));
    expect(router.state.location.pathname).toContain("/posts/01JB");
    expect(text).toHaveValue("Not saved yet");
  });
});

describe("the editor", () => {
  it("opens a post by its identity with its title and markdown", async () => {
    signedIn();
    const fetch = serve(() => json(200, { posts: [post(CALLOUT)] }));
    renderAt("/posts/01JB");
    expect(await screen.findByLabelText(en.editor.title)).toHaveValue("Planning a topic cluster");
    expect(screen.getByRole("textbox", { name: en.editor.text })).toHaveValue(CALLOUT);
    expect(fetch.mock.calls[0]?.[0].url).toContain("/api/admin/v1/posts/01JB/");
    expect(await axeViolations()).toEqual([]);
  });

  it("says when the post does not exist", async () => {
    signedIn();
    serve(() => json(404, {}));
    renderAt("/posts/missing");
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
      en.problems.notFound.what,
    );
  });

  it("says when the blog cannot be reached", async () => {
    signedIn();
    serve(() => {
      throw new TypeError("Failed to fetch");
    });
    renderAt("/posts/01JB");
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
      en.problems.unreachable.what,
    );
  });
});
