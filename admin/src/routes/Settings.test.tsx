import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import type { AdminPost, PostEdit } from "../api/posts";
import { en } from "../strings/en";

const POST: AdminPost = {
  id: "01JB",
  slug: "planning-a-topic-cluster",
  title: "Planning a topic cluster",
  status: "draft",
  author: "asif",
  markdown: "A topic cluster is a group of posts.\n",
  updated_at: "2026-10-27T10:42:00Z",
  settings: {
    co_authors: [],
    featured: false,
    review_state: "draft",
    share_cards: {},
    aliases: [],
  },
};

const UNSUPPORTED = {
  type: "https://khalid.dev/problems/unsupported-setting",
  title: "This blog cannot store co_authors yet.",
  status: 422,
  detail: "Post files have no field for it, so the change was not saved.",
  fix: "Leave this setting as it was, and save again.",
};

/** A blog holding one post, which records every edit and answers it with `status`. */
function blog(status = 200) {
  const edits: PostEdit[] = [];
  serve(async (request) => {
    if (request.method !== "PUT") {
      return json(200, { posts: [POST] });
    }
    const body: { posts: PostEdit[] } = await request.json();
    edits.push(...body.posts);
    const saved = { ...POST, updated_at: "2026-10-27T10:44:00Z" };
    return status === 200 ? json(200, { posts: [saved] }) : json(status, UNSUPPORTED);
  });
  return edits;
}

describe("the post settings panel", () => {
  it("shows the post's settings, each with what it does", async () => {
    signedIn();
    blog();
    renderAt("/posts/01JB");
    const slug = await screen.findByLabelText(en.settings.slug);
    expect(slug).toHaveValue("planning-a-topic-cluster");
    expect(slug).toHaveAccessibleDescription(en.settings.slugHint);
    expect(screen.getByLabelText(en.settings.author)).toHaveValue("asif");
    expect(await axeViolations()).toEqual([]);
  });

  it("saves only the settings that changed, with the text, on blur", async () => {
    signedIn();
    const edits = blog();
    renderAt("/posts/01JB");
    const slug = await screen.findByLabelText(en.settings.slug);
    fireEvent.change(slug, { target: { value: "plan-a-topic-cluster" } });
    const excerpt = screen.getByLabelText(en.settings.excerpt);
    fireEvent.change(excerpt, { target: { value: "One main post and several short ones." } });
    expect(excerpt).toHaveAccessibleDescription(en.settings.excerptHint(37));
    fireEvent.blur(excerpt);
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent(en.editor.saved));
    expect(edits).toEqual([
      {
        updated_at: "2026-10-27T10:42:00Z",
        title: POST.title,
        markdown: POST.markdown,
        slug: "plan-a-topic-cluster",
        settings: { excerpt: "One main post and several short ones." },
      },
    ]);
  });

  it("explains a setting the blog cannot store yet", async () => {
    signedIn();
    const edits = blog(422);
    renderAt("/posts/01JB");
    const others = await screen.findByLabelText(en.settings.coAuthors);
    fireEvent.change(others, { target: { value: "lena-vogt" } });
    fireEvent.blur(others);
    expect(await screen.findByRole("alert")).toHaveTextContent(UNSUPPORTED.title);
    expect(edits[0]?.settings).toEqual({ co_authors: ["lena-vogt"] });
    expect(screen.getByRole("status")).toHaveTextContent(en.editor.unsaved);
  });
});
