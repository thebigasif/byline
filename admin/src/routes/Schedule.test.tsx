import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import type { AdminPost, PostEdit } from "../api/posts";
import { localInput } from "../editor/schedule";
import { day } from "../strings/dates";
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
    review_state: "ready",
    access: "public",
    password_protected: false,
    share_cards: {},
    aliases: [],
  },
};

const REFUSED = "This blog makes no preview links.";

const LINK = {
  url: "https://www.example.com/blog/preview/en.planning/?until=1793696400&sig=c2ln",
  expires_at: "2026-11-03T10:42:00Z",
};

/** A blog holding `post`; edits come back applied, and preview links answer `preview`. */
function blog(post: AdminPost = DRAFT, preview = 201) {
  const edits: PostEdit[] = [];
  serve(async (request) => {
    if (request.url.endsWith("/preview-link/")) {
      return preview === 201
        ? json(201, { preview_links: [LINK] })
        : json(preview, {
            title: REFUSED,
            detail: "Preview links need a signing key, and this site has none.",
            fix: "Run khalid serve once more; it makes the key on start.",
          });
    }
    if (request.method !== "PUT") {
      return json(200, { posts: [post] });
    }
    const body: { posts: PostEdit[] } = await request.json();
    const edit = body.posts[0] ?? { updated_at: "" };
    edits.push(edit);
    const status = edit.status === "scheduled" ? "scheduled" : post.status;
    return json(200, { posts: [{ ...post, status, updated_at: "2026-10-27T10:50:00Z" }] });
  });
  return edits;
}

describe("going live at a time", () => {
  it("schedules the post for the time picked, in the writer's time zone", async () => {
    signedIn();
    const edits = blog();
    renderAt("/posts/01JB");
    await userEvent.click(await screen.findByLabelText(en.goLive.later));
    const at = screen.getByLabelText(en.goLive.at);
    fireEvent.change(at, { target: { value: "2026-11-03T09:00" } });
    await userEvent.click(screen.getByRole("button", { name: en.editor.schedule }));
    await waitFor(() => expect(screen.getByText(en.editor.scheduled)).toBeVisible());
    expect(edits).toEqual([
      {
        updated_at: "2026-10-27T10:42:00Z",
        status: "scheduled",
        published_at: new Date("2026-11-03T09:00").toISOString(),
      },
    ]);
    expect(await axeViolations()).toEqual([]);
  });

  it("opens a scheduled post with its time picked", async () => {
    signedIn();
    blog({ ...DRAFT, status: "scheduled", published_at: "2026-11-03T08:00:00Z" });
    renderAt("/posts/01JB");
    expect(await screen.findByLabelText(en.goLive.later)).toBeChecked();
    expect(screen.getByLabelText(en.goLive.at)).toHaveValue(localInput("2026-11-03T08:00:00Z"));
    expect(screen.getByRole("button", { name: en.editor.schedule })).toBeVisible();
  });

  it("publishes at once when the writer goes back to now", async () => {
    signedIn();
    const edits = blog();
    renderAt("/posts/01JB");
    await userEvent.click(await screen.findByLabelText(en.goLive.later));
    await userEvent.click(screen.getByLabelText(en.goLive.now));
    expect(screen.queryByLabelText(en.goLive.at)).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: en.editor.publish }));
    await waitFor(() => expect(edits[0]?.status).toBe("published"));
  });
});

describe("a preview link", () => {
  it("makes a link anyone can read the draft with, and says until when", async () => {
    signedIn();
    blog();
    renderAt("/posts/01JB");
    await userEvent.click(await screen.findByRole("button", { name: en.preview.make }));
    const field = await screen.findByLabelText(en.preview.label);
    expect(field).toHaveValue(LINK.url);
    expect(field).toHaveAttribute("readonly");
    expect(field).toHaveAccessibleDescription(new RegExp(day(LINK.expires_at)));
  });

  it("explains a blog that makes no preview links", async () => {
    signedIn();
    blog(DRAFT, 422);
    renderAt("/posts/01JB");
    await userEvent.click(await screen.findByRole("button", { name: en.preview.make }));
    expect(await screen.findByRole("alert")).toHaveTextContent(REFUSED);
  });
});
