import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import type { SchemaTemplates } from "../api/schemaTemplates";
import { en } from "../strings/en";

const COURSE = {
  applies_to: "collection" as const,
  name: "guides",
  type: "Course",
  properties: { provider: "{site}" },
};

const HELD: SchemaTemplates = {
  templates: [COURSE],
  collections: ["guides", "posts"],
  updated_at: "2026-10-30T09:00:00Z",
  restart_needed: false,
};

const REFUSED = {
  type: "https://khalid.dev/problems/invalid-body",
  title: "The request could not be read",
  status: 422,
  detail:
    'The schema templates could not be read: template 2: "recipe" is not a schema.org type name such as Course.',
  fix: "Correct the template the message names, then save again.",
};

/** A blog holding `HELD`, which records each change and answers it with `status`. */
function blog(status = 200) {
  const changes: unknown[] = [];
  serve(async (request) => {
    if (request.method !== "PUT") {
      return json(200, HELD);
    }
    const body: { templates: unknown[] } = await request.json();
    changes.push(body);
    if (status !== 200) {
      return json(status, REFUSED);
    }
    return json(200, { ...body, collections: HELD.collections, restart_needed: true });
  });
  return changes;
}

/** Fills the form with a Recipe for every post, named after its title. */
async function addRecipe(type = "Recipe") {
  const collection = await screen.findByLabelText(en.templates.form.collection);
  fireEvent.change(collection, { target: { value: "posts" } });
  const form = en.templates.form;
  fireEvent.change(screen.getByLabelText(form.type), { target: { value: type } });
  fireEvent.change(screen.getByLabelText(form.name), { target: { value: "name" } });
  fireEvent.change(screen.getByLabelText(form.value), { target: { value: "{title}" } });
  fireEvent.click(screen.getByRole("button", { name: en.templates.form.save }));
}

describe("the schema templates screen", () => {
  it("lists the templates in use and adds one beside them", async () => {
    signedIn();
    const changes = blog();
    renderAt("/schema-templates");
    const heading = en.templates.every("collection", "guides", "Course");
    expect(await screen.findByRole("heading", { name: heading })).toBeVisible();
    expect(await axeViolations()).toEqual([]);
    await addRecipe();
    expect(await screen.findByText(en.templates.restart)).toBeVisible();
    const recipe = { applies_to: "collection", name: "posts", type: "Recipe" };
    expect(changes).toEqual([
      {
        templates: [COURSE, { ...recipe, properties: { name: "{title}" } }],
        updated_at: HELD.updated_at,
      },
    ]);
    const added = en.templates.every("collection", "posts", "Recipe");
    expect(screen.getByRole("heading", { name: added })).toBeVisible();
  });

  it("says which template the blog refused, and keeps the list", async () => {
    signedIn();
    blog(422);
    renderAt("/schema-templates");
    await addRecipe("recipe");
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("template 2"));
    const heading = en.templates.every("collection", "guides", "Course");
    expect(screen.getByRole("heading", { name: heading })).toBeVisible();
  });

  it("removes a template by saving the list without it", async () => {
    signedIn();
    const changes = blog();
    renderAt("/schema-templates");
    fireEvent.click(await screen.findByRole("button", { name: en.templates.remove }));
    expect(await screen.findByText(en.templates.none)).toBeVisible();
    expect(changes).toEqual([{ templates: [], updated_at: HELD.updated_at }]);
  });
});
