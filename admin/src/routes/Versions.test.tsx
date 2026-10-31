import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import type { AdminPost, PostEdit } from "../api/posts";
import type { PostVersion } from "../api/versions";
import { en } from "../strings/en";

const DRAFT: AdminPost = {
  id: "01JB",
  slug: "why-internal-links-matter",
  title: "Why internal links matter",
  status: "draft",
  markdown: "Internal links help readers and Google find the rest of your site.\n",
  updated_at: "2026-10-31T10:42:00Z",
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

const EARLIER: PostVersion = {
  saved_at: "2026-10-30T16:05:00Z",
  saved_by: "maria",
  title: "Internal links",
  markdown: "Internal links help Google.\n",
};

/** A blog holding DRAFT with `versions` kept, recording each edit it is sent. */
function blog(versions: PostVersion[]) {
  const edits: PostEdit[] = [];
  serve(async (request) => {
    if (request.url.endsWith("/versions/")) {
      return json(200, { versions });
    }
    if (request.method !== "PUT") {
      return json(200, { posts: [DRAFT] });
    }
    const body: { posts: PostEdit[] } = await request.json();
    edits.push(...body.posts);
    return json(200, { posts: [{ ...DRAFT, updated_at: "2026-10-31T10:50:00Z" }] });
  });
  return edits;
}

const showVersions = async () => {
  renderAt("/posts/01JB");
  fireEvent.click(await screen.findByRole("button", { name: en.versions.show }));
};

describe("earlier versions", () => {
  it("lists them with who saved each, and shows what changed since one", async () => {
    signedIn();
    blog([EARLIER]);
    await showVersions();
    const list = await screen.findByRole("list", { name: en.versions.list });
    const pick = within(list).getByRole("button", { name: /by maria$/ });
    fireEvent.click(pick);
    expect(pick).toHaveAttribute("aria-pressed", "true");
    const changes = screen.getByRole("group", { name: en.versions.changes });
    expect(within(changes).getByText("Internal links help Google.").tagName).toBe("DEL");
    const now = within(changes).getByText(DRAFT.markdown.trim());
    expect(now.tagName).toBe("INS");
    expect(await axeViolations()).toEqual([]);
  });

  it("brings one back into the editor and saves it", async () => {
    signedIn();
    const edits = blog([EARLIER]);
    await showVersions();
    const list = await screen.findByRole("list", { name: en.versions.list });
    fireEvent.click(within(list).getByRole("button", { name: /by maria$/ }));
    fireEvent.click(screen.getByRole("button", { name: en.versions.restore }));
    const title = screen.getByLabelText(en.editor.title);
    expect(title).toHaveValue(EARLIER.title);
    fireEvent.blur(title);
    await waitFor(() => expect(edits).toHaveLength(1));
    expect(edits[0]?.title).toBe(EARLIER.title);
    expect(edits[0]?.markdown).toBe(EARLIER.markdown);
  });

  it("says so when none are kept yet", async () => {
    signedIn();
    blog([]);
    await showVersions();
    expect(await screen.findByText(en.versions.none)).toBeVisible();
  });
});
