import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { adminBlog, noContent } from "../../test/blog";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { AdminKey } from "../api/keys";
import { en } from "../strings/en";

/** The three keys from the mockup, as the API lists them: never with a secret. */
const KEPT: AdminKey[] = [
  { id: "1111", name: "Web page", kind: "content", scopes: [], created_at: "2026-10-20T09:00:00Z" },
  {
    id: "2222",
    name: "SerpDad",
    kind: "integration",
    scopes: ["posts:write", "media:write"],
    created_at: "2026-10-21T09:00:00Z",
  },
  {
    id: "3333",
    name: "Lena",
    kind: "staff",
    role: "editor",
    scopes: [],
    created_at: "2026-10-22T09:00:00Z",
  },
];

/** A blog that keeps the three keys and switches off the one it is asked to. */
function blog(answer: () => Response = noContent) {
  let list = [...KEPT];
  return adminBlog({
    "GET /keys/": () => json(200, { keys: list }),
    "DELETE /keys/2222/": () => {
      const reply = answer();
      if (reply.ok) {
        list = list.filter((key) => key.id !== "2222");
      }
      return reply;
    },
  });
}

describe("the API keys screen", () => {
  it("lists each key with its kind and what it may do, in words", async () => {
    signedIn();
    blog();
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.keys }));
    const table = await screen.findByRole("table", { name: en.keys.list.title });
    const [, page, serpdad, lena] = within(table).getAllByRole("row");
    expect(page).toHaveTextContent(en.keys.kinds.content);
    expect(page).toHaveTextContent(en.keys.scopes["content:read"]);
    expect(serpdad).toHaveTextContent("Write drafts, Upload images and files");
    expect(lena).toHaveTextContent(en.keys.asRole("editor"));
    expect(await axeViolations()).toEqual([]);
  });

  it("switches a key off, and it leaves the list", async () => {
    signedIn();
    const sent = blog();
    renderAt("/keys");
    const revoke = en.keys.list.revokeLabel("SerpDad");
    await userEvent.click(await screen.findByRole("button", { name: revoke }));
    expect(await screen.findByText(en.keys.list.revoked("SerpDad"))).toBeVisible();
    await waitFor(() => expect(screen.queryByText("SerpDad")).toBeNull());
    expect(sent.filter((request) => request.method === "DELETE")).toHaveLength(1);
  });

  it("explains why the key in use cannot be switched off", async () => {
    signedIn();
    const own = {
      type: "https://khalid.dev/problems/conflict",
      title: "This is the key you are using.",
      status: 409,
      detail: "Switching it off would sign you out with no way back in.",
      fix: "Sign in with another key first, then switch this one off.",
    };
    blog(() => json(409, own));
    renderAt("/keys");
    const revoke = en.keys.list.revokeLabel("SerpDad");
    await userEvent.click(await screen.findByRole("button", { name: revoke }));
    expect(await screen.findByRole("alert")).toHaveTextContent(own.title);
    expect(screen.getByText("SerpDad")).toBeVisible();
  });
});
