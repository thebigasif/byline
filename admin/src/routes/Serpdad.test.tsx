import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import type { SerpdadDraft, SerpdadState } from "../api/serpdad";
import { en } from "../strings/en";

// Made up for these tests; it opens nothing.
const KEY = "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const AT = "2026-10-28T10:12:00Z";

function draft(fields: Partial<SerpdadDraft> = {}): SerpdadDraft {
  return {
    id: "en.choosing-anchor-text",
    title: "Choosing anchor text",
    lint: { blocks: 0, warnings: 2 },
    received_at: "2026-10-31T09:40:00Z",
    admin_path: "/admin/posts/en.choosing-anchor-text",
    ...fields,
  };
}

/** A blog that keeps the SerpDad connection as the API does, starting from `start`. */
function blog(start: SerpdadState) {
  let state = start;
  const sent: string[] = [];
  serve((request) => {
    sent.push(request.method);
    const route = new URL(request.url).pathname;
    if (route !== "/api/admin/v1/serpdad/") {
      return json(404, {});
    }
    if (request.method === "POST") {
      state = { ...state, connection: { connected_at: AT } };
      return json(201, { ...state, connection: { connected_at: AT, key: KEY } });
    }
    if (request.method === "DELETE") {
      state = { ...state, connection: null };
      return new Response(null, { status: 204 });
    }
    return json(200, state);
  });
  return sent;
}

describe("Connect to SerpDad", () => {
  it("is off at first and explains the key before it is made", async () => {
    signedIn();
    blog({ connection: null, drafts: [] });
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.serpdad }));
    expect(await screen.findByRole("heading", { level: 1, name: en.serpdad.title })).toBeVisible();
    expect(await screen.findByText(en.serpdad.off.what)).toBeVisible();
    expect(screen.getByRole("button", { name: en.serpdad.off.make })).toBeEnabled();
    expect(screen.queryByRole("table")).toBeNull();
    expect(screen.getByRole("link", { name: en.nav.serpdad })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(await axeViolations()).toEqual([]);
  });

  it("shows the new key once, beside the blog's address", async () => {
    signedIn();
    // setup() gives the page a clipboard that remembers what was written to it.
    const user = userEvent.setup();
    const sent = blog({ connection: null, drafts: [] });
    const { unmount } = renderAt("/serpdad");
    await user.click(await screen.findByRole("button", { name: en.serpdad.off.make }));
    expect(await screen.findByLabelText(en.serpdad.key.key)).toHaveValue(KEY);
    expect(screen.getByLabelText(en.serpdad.key.address)).toHaveValue(globalThis.location.origin);
    expect(sent).toContain("POST");
    await user.click(screen.getByRole("button", { name: en.serpdad.key.copy }));
    expect(await screen.findByText(en.serpdad.key.copied)).toBeVisible();
    expect(await navigator.clipboard.readText()).toBe(KEY);
    expect(await screen.findByText(en.serpdad.drafts.none)).toBeVisible();
    expect(await axeViolations()).toEqual([]);

    unmount();
    renderAt("/serpdad");
    expect(await screen.findByText(en.serpdad.connected)).toBeVisible();
    expect(screen.queryByLabelText(en.serpdad.key.key)).toBeNull();
  });

  it("lists the drafts SerpDad sent, each opening in the editor with its checks", async () => {
    signedIn();
    const weak = draft({ id: "en.a-weak-post", title: "A weak post" });
    const unchecked = draft({ id: "en.unchecked", title: "Not linted", lint: null });
    blog({
      connection: { connected_at: AT },
      drafts: [{ ...weak, lint: { blocks: 1, warnings: 3 } }, draft(), unchecked],
    });
    renderAt("/serpdad");
    const table = await screen.findByRole("table", { name: en.serpdad.drafts.title });
    const [, first, second, third] = within(table).getAllByRole("row");
    const link = within(table).getByRole("link", { name: "A weak post" });
    expect(link).toHaveAttribute("href", "/admin/posts/en.a-weak-post");
    expect(first).toHaveTextContent(en.posts.blocks(1));
    expect(second).toHaveTextContent(en.posts.warnings(2));
    expect(third).toHaveTextContent(en.serpdad.drafts.unchecked);
    expect(screen.queryByRole("button", { name: en.serpdad.off.make })).toBeNull();
    expect(await axeViolations()).toEqual([]);
  });

  it("turns off and keeps the drafts", async () => {
    signedIn();
    const sent = blog({ connection: { connected_at: AT }, drafts: [draft()] });
    renderAt("/serpdad");
    await userEvent.click(await screen.findByRole("button", { name: en.serpdad.change.off }));
    expect(await screen.findByText(en.serpdad.change.turnedOff)).toBeVisible();
    expect(await screen.findByRole("button", { name: en.serpdad.off.make })).toBeVisible();
    expect(screen.getByRole("table", { name: en.serpdad.drafts.title })).toBeVisible();
    expect(sent).toContain("DELETE");
  });

  it("shows the API's words when the key may not change settings", async () => {
    signedIn();
    const lacks = {
      type: "https://khalid.dev/problems/forbidden",
      title: "This key does not have the settings:write scope.",
      status: 403,
      detail: "Each key may do only what it was made for, so a leaked key does less harm.",
      fix: "Use a key with settings:write, or make one with that scope.",
    };
    serve(() => json(403, lacks));
    renderAt("/serpdad");
    expect(await screen.findByText(lacks.title)).toBeVisible();
    expect(screen.getByText(lacks.fix)).toBeVisible();
    expect(screen.queryByRole("button", { name: en.serpdad.off.make })).toBeNull();
  });
});
