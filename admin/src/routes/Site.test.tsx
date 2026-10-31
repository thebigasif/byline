import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { adminBlog } from "../../test/blog";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { SiteChange, SiteSettings } from "../api/site";
import { en } from "../strings/en";

// The demo site's real site.toml.
const DEMO: SiteSettings = {
  name: "Field Notes on SEO",
  address: "https://www.example.com",
  mount: "/blog/",
  default_language: "en",
  languages: ["en", "de"],
  updated_at: "2026-10-30T09:00:00Z",
  restart_needed: false,
};

/** A blog with the demo settings that records each change and then needs a restart. */
function blog() {
  const changes: SiteChange[] = [];
  let site = DEMO;
  adminBlog({
    "GET /settings/": () => json(200, { settings: site }),
    "PUT /settings/": async (request) => {
      const body: { settings: SiteChange } = await request.json();
      changes.push(body.settings);
      site = { ...DEMO, name: body.settings.name ?? DEMO.name, restart_needed: true };
      return json(200, { settings: site });
    },
  });
  return changes;
}

const FIELDS = en.site.fields;

describe("the Settings screen", () => {
  it("shows the settings with the warning beside the address", async () => {
    signedIn();
    blog();
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.settings }));
    expect(await screen.findByLabelText(FIELDS.name.label)).toHaveValue(DEMO.name);
    expect(screen.getByLabelText(FIELDS.mount.label)).toHaveValue("/blog/");
    expect(screen.getByText(en.site.careful)).toBeVisible();
    const main = screen.getByLabelText(FIELDS.default_language.label);
    expect(main).toHaveDisplayValue("English");
    expect(screen.queryByText(en.site.restart)).toBeNull();
    expect(await axeViolations()).toEqual([]);
  });

  it("saves only the changed name, and says a restart applies it", async () => {
    signedIn();
    const changes = blog();
    renderAt("/settings");
    const name = await screen.findByLabelText(FIELDS.name.label);
    await userEvent.clear(name);
    await userEvent.type(name, "Field Notes");
    await userEvent.click(screen.getByRole("button", { name: en.site.save }));
    expect(await screen.findByText(en.site.saved)).toBeVisible();
    expect(changes).toEqual([{ updated_at: DEMO.updated_at, name: "Field Notes" }]);
    expect(await screen.findByText(en.site.restart)).toBeVisible();
  });

  it("sends the languages as a list, and nothing when nothing changed", async () => {
    signedIn();
    const changes = blog();
    renderAt("/settings");
    await userEvent.click(await screen.findByRole("button", { name: en.site.save }));
    expect(await screen.findByText(en.site.unchanged)).toBeVisible();
    await userEvent.type(screen.getByLabelText(FIELDS.languages.label), ", FR");
    await userEvent.click(screen.getByRole("button", { name: en.site.save }));
    expect(await screen.findByText(en.site.saved)).toBeVisible();
    expect(changes).toEqual([{ updated_at: DEMO.updated_at, languages: ["en", "de", "fr"] }]);
  });
});
