import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { kept, redirectsBlog } from "../../test/redirects";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { StoredRedirect } from "../api/redirects";
import { en } from "../strings/en";

const PATTERN = kept({ id: 13, from: "^/2019/(.*)$", to: "/blog/$1", match: "regex" });
const GONE = kept({ id: 14, from: "/blog/old-offer/", to: null, status: 410, origin: "import" });

/** A blog that keeps `stored` and removes from it on request. */
function keeping(stored: StoredRedirect[]) {
  let list = [...stored];
  return redirectsBlog({
    "GET /redirects/": () => json(200, { redirects: list }),
    "DELETE /redirects/13/": () => {
      list = list.filter((redirect) => redirect.id !== 13);
      return new Response(null, { status: 204 });
    },
  });
}

describe("the redirects Khalid keeps", () => {
  it("lists each one with where it leads, its kind and who made it", async () => {
    signedIn();
    keeping([kept({ origin: "slug_change" }), PATTERN, GONE]);
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.redirects }));
    const heading = await screen.findByRole("heading", { level: 1, name: en.redirects.title });
    expect(heading).toBeVisible();
    const table = await screen.findByRole("table", { name: en.redirects.kept.title });
    const [, moved, pattern, gone] = within(table).getAllByRole("row");
    expect(moved).toHaveTextContent("/blog/why-internal-links-matter/");
    expect(moved).toHaveTextContent(en.redirects.kind(301, false));
    expect(moved).toHaveTextContent(en.redirects.origins.slug_change);
    expect(pattern).toHaveTextContent(en.redirects.kind(301, true));
    expect(gone).toHaveTextContent(en.redirects.kept.nothing);
    expect(gone).toHaveTextContent(en.redirects.kind(410, false));
    expect(gone).toHaveTextContent(en.redirects.origins.import);
    expect(screen.getByRole("link", { name: en.nav.redirects })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(await axeViolations()).toEqual([]);
  });

  it("removes one and lists the rest again", async () => {
    signedIn();
    const sent = keeping([kept(), PATTERN]);
    renderAt("/redirects");
    const remove = await screen.findByRole("button", {
      name: en.redirects.kept.remove(PATTERN.from),
    });
    await userEvent.click(remove);
    expect(await screen.findByText(en.redirects.kept.removed(PATTERN.from))).toBeVisible();
    await waitFor(() => expect(screen.queryByText(PATTERN.from)).toBeNull());
    expect(sent.filter((request) => request.method === "DELETE")).toHaveLength(1);
    expect(screen.getByText(kept().from)).toBeVisible();
  });

  it("says when Khalid keeps none", async () => {
    signedIn();
    keeping([]);
    renderAt("/redirects");
    expect(await screen.findByText(en.redirects.kept.none)).toBeVisible();
    expect(screen.queryByRole("table", { name: en.redirects.kept.title })).toBeNull();
  });

  it("shows the API's words when the key may not change redirects", async () => {
    signedIn();
    const lacks = {
      type: "https://khalid.dev/problems/forbidden",
      title: "This key cannot change site settings.",
      status: 403,
      detail: "Redirects need the settings:write scope.",
      fix: "Sign in with an owner or admin staff token.",
    };
    redirectsBlog({ "GET /redirects/": () => json(403, lacks) });
    renderAt("/redirects");
    expect(await screen.findByRole("alert")).toHaveTextContent(lacks.title);
  });
});
