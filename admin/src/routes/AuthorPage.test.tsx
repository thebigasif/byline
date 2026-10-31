import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { type Answers, adminBlog, noContent } from "../../test/blog";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { AdminAuthor, AuthorEdit } from "../api/authors";
import { en } from "../strings/en";

const LENA: AdminAuthor = {
  id: "de.lena-vogt",
  slug: "lena-vogt",
  language: "de",
  name: "Lena Vogt",
  role: "Technical SEO editor",
  bio: "Lena looks after the technical side.\n",
  same_as: [],
  updated_at: "2026-10-30T09:00:00Z",
};

/** A blog with Lena that records every edit, then answers with `more`. */
function blog(more: Answers = {}) {
  const edits: AuthorEdit[] = [];
  const sent = adminBlog({
    "GET /authors/": () => json(200, { authors: [LENA] }),
    "GET /authors/de.lena-vogt/": () => json(200, { authors: [LENA] }),
    "PUT /authors/de.lena-vogt/": async (request) => {
      const body: { authors: AuthorEdit[] } = await request.json();
      edits.push(...body.authors);
      return json(200, { authors: [{ ...LENA, updated_at: "2026-10-31T10:00:00Z" }] });
    },
    ...more,
  });
  return { sent, edits };
}

const FIELDS = en.authors.edit.fields;

describe("an author's page", () => {
  it("opens from the list and saves only the changed fields", async () => {
    signedIn();
    const { edits } = blog();
    renderAt("/authors");
    const edit = en.authors.list.editLabel("Lena Vogt", "German");
    await userEvent.click(await screen.findByRole("link", { name: edit }));
    expect(await screen.findByRole("heading", { level: 1, name: "Lena Vogt" })).toBeVisible();
    await userEvent.type(screen.getByLabelText(FIELDS.credentials.label), "Eight years in SEO");
    await userEvent.type(screen.getByLabelText(FIELDS.sameAs.label), "https://example.com/lena");
    await userEvent.click(screen.getByRole("button", { name: en.authors.edit.save }));
    expect(await screen.findByText(en.authors.edit.saved)).toBeVisible();
    expect(edits).toEqual([
      {
        updated_at: LENA.updated_at,
        credentials: "Eight years in SEO",
        same_as: ["https://example.com/lena"],
      },
    ]);
    expect(await axeViolations()).toEqual([]);
  });

  it("shows the API's words when someone changed the author first", async () => {
    signedIn();
    const stale = {
      type: "https://khalid.dev/problems/conflict",
      title: "Someone else changed this author first.",
      status: 409,
      detail: "Saving now would undo their change.",
      fix: "Reload the page and make your change again.",
    };
    blog({ "PUT /authors/de.lena-vogt/": () => json(409, stale) });
    renderAt("/authors/de.lena-vogt");
    await userEvent.clear(await screen.findByLabelText(FIELDS.role.label));
    await userEvent.click(screen.getByRole("button", { name: en.authors.edit.save }));
    expect(await screen.findByRole("alert")).toHaveTextContent(stale.title);
  });

  it("deletes the author with the copy's time, and goes back to the list", async () => {
    signedIn();
    const { sent } = blog({ "DELETE /authors/de.lena-vogt/": noContent });
    renderAt("/authors/de.lena-vogt");
    const remove = en.authors.edit.remove.submit("Lena Vogt");
    await userEvent.click(await screen.findByRole("button", { name: remove }));
    expect(await screen.findByRole("heading", { level: 1, name: en.authors.title })).toBeVisible();
    const deleted = sent.find((request) => request.method === "DELETE");
    expect(deleted?.url).toContain("updated_at=2026-10-30T09%3A00%3A00Z");
  });
});
