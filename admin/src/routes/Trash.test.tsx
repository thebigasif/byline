import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { redirectsBlog } from "../../test/redirects";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { TrashedPost } from "../api/records";
import { en } from "../strings/en";

const LIVE: TrashedPost = {
  key: "20261031T093000Z-en.what-a-sitemap-does",
  id: "en.what-a-sitemap-does",
  title: "What a sitemap does",
  status: "published",
  trashed_at: "2026-10-31T09:30:00Z",
};
const DRAFT: TrashedPost = {
  key: "20261030T160500Z-en.choosing-anchor-text",
  id: "en.choosing-anchor-text",
  title: "Choosing anchor text",
  status: "draft",
  trashed_at: "2026-10-30T16:05:00Z",
};

/** A blog whose trash holds `held`, restoring and purging on request. */
function trashing(held: TrashedPost[]) {
  let trash = [...held];
  const without = (key: string) => {
    trash = trash.filter((post) => post.key !== key);
  };
  return redirectsBlog({
    "GET /trash/": () => json(200, { trash }),
    [`POST /trash/${LIVE.key}/restore/`]: () => {
      without(LIVE.key);
      return json(200, { posts: [{ id: LIVE.id }] });
    },
    [`DELETE /trash/${DRAFT.key}/`]: () => {
      without(DRAFT.key);
      return new Response(null, { status: 204 });
    },
  });
}

describe("the trash", () => {
  it("lists each deleted post with what it was and when it went", async () => {
    signedIn();
    trashing([LIVE, DRAFT]);
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.trash }));
    expect(await screen.findByRole("heading", { level: 1, name: en.trash.title })).toBeVisible();
    const table = await screen.findByRole("table", { name: en.trash.title });
    const [, live, draft] = within(table).getAllByRole("row");
    expect(live).toHaveTextContent(LIVE.title);
    expect(live).toHaveTextContent(en.trash.status.published);
    expect(draft).toHaveTextContent(en.trash.status.draft);
    expect(screen.getByText(en.trash.count(2))).toBeVisible();
    expect(await axeViolations()).toEqual([]);
  });

  it("puts a post back and lists the rest again", async () => {
    signedIn();
    const sent = trashing([LIVE, DRAFT]);
    renderAt("/trash");
    const restore = await screen.findByRole("button", { name: en.trash.restore(LIVE.title) });
    await userEvent.click(restore);
    expect(await screen.findByText(en.trash.restored(LIVE.title))).toBeVisible();
    await waitFor(() => expect(screen.queryByRole("cell", { name: LIVE.title })).toBeNull());
    expect(sent.filter((request) => request.method === "POST")).toHaveLength(1);
  });

  it("asks once more before it deletes a post for good", async () => {
    signedIn();
    const sent = trashing([DRAFT]);
    renderAt("/trash");
    const purge = await screen.findByRole("button", { name: en.trash.purge(DRAFT.title) });
    await userEvent.click(purge);
    expect(screen.getByText(en.trash.asking(DRAFT.title))).toBeVisible();
    expect(sent.filter((request) => request.method === "DELETE")).toHaveLength(0);
    await userEvent.click(screen.getByRole("button", { name: en.trash.sure(DRAFT.title) }));
    expect(await screen.findByText(en.trash.purged(DRAFT.title))).toBeVisible();
    expect(await screen.findByText(en.trash.none)).toBeVisible();
  });

  it("says what went wrong when the trash cannot be read", async () => {
    signedIn();
    redirectsBlog({ "GET /trash/": () => json(500, {}) });
    renderAt("/trash");
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.unreachable.what);
  });
});
