import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { adminBlog } from "../../test/blog";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { AdminAuthor, NewAuthor } from "../api/authors";
import { en } from "../strings/en";

/** The demo site's Asif in English, unless `fields` say otherwise. */
function author(fields: Partial<AdminAuthor> = {}): AdminAuthor {
  return {
    id: "en.asif",
    slug: "asif",
    language: "en",
    name: "Asif",
    role: "Founder of SerpDad",
    credentials: "Ten years running SEO for small teams",
    bio: "Asif writes about search.\n",
    same_as: ["https://serpdad.com/"],
    updated_at: "2026-10-30T09:00:00Z",
    ...fields,
  };
}

const LENA = author({ id: "de.lena-vogt", slug: "lena-vogt", language: "de", name: "Lena Vogt" });

/** A blog that keeps `kept` and adds every author it is sent. */
function blog(kept: AdminAuthor[]) {
  let list = [...kept];
  const made: NewAuthor[] = [];
  adminBlog({
    "GET /authors/": () => json(200, { authors: list }),
    "POST /authors/": async (request) => {
      const body: { authors: NewAuthor[] } = await request.json();
      made.push(...body.authors);
      const [{ name, slug } = author()] = body.authors;
      const added = author({ id: `en.${slug}`, name, slug, role: null });
      list = [...list, added];
      return json(201, { authors: [added] });
    },
  });
  return made;
}

const ADD = en.authors.add.fields;

describe("the Authors screen", () => {
  it("lists every author with their language by name", async () => {
    signedIn();
    blog([author(), LENA]);
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.authors }));
    const table = await screen.findByRole("table", { name: en.authors.list.title });
    const [, asif, lena] = within(table).getAllByRole("row");
    expect(asif).toHaveTextContent("Founder of SerpDad");
    expect(asif).toHaveTextContent("English");
    expect(lena).toHaveTextContent("German");
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(await axeViolations()).toEqual([]);
  });

  it("adds an author, leaving out an empty role and language", async () => {
    signedIn();
    const made = blog([author()]);
    renderAt("/authors");
    await userEvent.type(await screen.findByLabelText(ADD.name.label), "Mira Hoffmann");
    await userEvent.type(screen.getByLabelText(ADD.slug.label), "mira-hoffmann");
    await userEvent.click(screen.getByRole("button", { name: en.authors.add.submit }));
    expect(await screen.findByText(en.authors.add.added("Mira Hoffmann"))).toBeVisible();
    expect(made).toEqual([
      { name: "Mira Hoffmann", slug: "mira-hoffmann", role: null, language: null },
    ]);
    expect(await screen.findByText(en.authors.list.noRole)).toBeVisible();
  });

  it("says when this server cannot keep authors yet", async () => {
    signedIn();
    adminBlog({});
    renderAt("/authors");
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.notYet.what);
  });
});
