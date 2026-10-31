import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { redirectsBlog } from "../../test/redirects";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { Passkey } from "../api/passkeys";
import { en } from "../strings/en";

const LAPTOP: Passkey = {
  id: 1,
  name: "Work laptop",
  added_at: "2026-10-27T09:00:00Z",
  last_used_at: "2026-10-31T08:42:00Z",
};
const PHONE: Passkey = {
  id: 2,
  name: "Phone",
  added_at: "2026-10-28T19:15:00Z",
  last_used_at: null,
};

/** A blog where the signed-in person has `held`, renaming and removing on request. */
function holding(held: Passkey[]) {
  let passkeys = [...held];
  return redirectsBlog({
    "GET /session/passkeys/": () => json(200, passkeys),
    "PATCH /session/passkeys/2/": async (request) => {
      const { name } = await request.clone().json();
      passkeys = passkeys.map((passkey) => (passkey.id === 2 ? { ...passkey, name } : passkey));
      return new Response(null, { status: 204 });
    },
    "DELETE /session/passkeys/1/": () => {
      passkeys = passkeys.filter((passkey) => passkey.id !== 1);
      return new Response(null, { status: 204 });
    },
  });
}

describe("the passkeys screen", () => {
  it("lists each passkey with when it was added and last used", async () => {
    signedIn();
    holding([LAPTOP, PHONE]);
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.passkeys }));
    expect(await screen.findByRole("heading", { level: 1, name: en.passkeys.title })).toBeVisible();
    const table = await screen.findByRole("table", { name: en.passkeys.listTitle });
    const [, laptop, phone] = within(table).getAllByRole("row");
    expect(laptop).toHaveTextContent(LAPTOP.name);
    expect(laptop).toHaveTextContent("27 October 2026");
    expect(phone).toHaveTextContent(en.passkeys.never);
    expect(screen.getByText(en.passkeys.count(2))).toBeVisible();
    expect(await axeViolations()).toEqual([]);
  });

  it("renames a passkey in its own row", async () => {
    signedIn();
    const sent = holding([LAPTOP, PHONE]);
    renderAt("/passkeys");
    await userEvent.click(await screen.findByRole("button", { name: en.passkeys.rename("Phone") }));
    const field = screen.getByLabelText(en.passkeys.newName("Phone"));
    await userEvent.clear(field);
    await userEvent.type(field, "  Pixel phone ");
    await userEvent.click(screen.getByRole("button", { name: en.passkeys.saveName }));
    expect(await screen.findByText(en.passkeys.renamed("Pixel phone"))).toBeVisible();
    expect(await screen.findByRole("cell", { name: "Pixel phone" })).toBeVisible();
    const patch = sent.find((request) => request.method === "PATCH");
    expect(await patch?.json()).toEqual({ name: "Pixel phone" });
  });

  it("refuses a blank name without asking the blog", async () => {
    signedIn();
    const sent = holding([PHONE]);
    renderAt("/passkeys");
    await userEvent.click(await screen.findByRole("button", { name: en.passkeys.rename("Phone") }));
    const field = screen.getByLabelText(en.passkeys.newName("Phone"));
    await userEvent.clear(field);
    await userEvent.type(field, "   ");
    await userEvent.click(screen.getByRole("button", { name: en.passkeys.saveName }));
    expect(await screen.findByText(en.passkeys.problems.blankName.what)).toBeVisible();
    expect(sent.filter((request) => request.method === "PATCH")).toHaveLength(0);
  });

  it("asks once more before it removes a passkey", async () => {
    signedIn();
    const sent = holding([LAPTOP, PHONE]);
    renderAt("/passkeys");
    const remove = await screen.findByRole("button", { name: en.passkeys.remove(LAPTOP.name) });
    await userEvent.click(remove);
    expect(screen.getByText(en.passkeys.asking(LAPTOP.name))).toBeVisible();
    expect(sent.filter((request) => request.method === "DELETE")).toHaveLength(0);
    await userEvent.click(screen.getByRole("button", { name: en.passkeys.sure(LAPTOP.name) }));
    expect(await screen.findByText(en.passkeys.removed(LAPTOP.name))).toBeVisible();
    await waitFor(() => expect(screen.queryByRole("cell", { name: LAPTOP.name })).toBeNull());
    expect(screen.getByText(en.passkeys.count(1))).toBeVisible();
  });

  it("tells a tab signed in with a key to sign in as a person", async () => {
    signedIn();
    redirectsBlog({ "GET /session/passkeys/": () => json(401, {}) });
    renderAt("/passkeys");
    expect(await screen.findByText(en.passkeys.problems.noSession.what)).toBeVisible();
    expect(screen.queryByRole("table")).toBeNull();
  });
});
