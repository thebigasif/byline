import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import { listing, row, rowsPage } from "../../test/rows";
import type { PostRow } from "../api/rows";
import { en } from "../strings/en";

const STUCK = row({
  id: "01JB",
  title: "Planning a topic cluster in an afternoon",
  status: "draft",
  gate: "refuses",
  lint: { blocks: 1, warnings: 2 },
});

/** A blog with five live posts, two drafts and the `blocked` posts, which records every request. */
function blog(blocked: PostRow[] = []) {
  const sent: Request[] = [];
  serve((request) => {
    sent.push(request);
    const query = listing(request);
    if (query === null) {
      return json(404, {});
    }
    const counts: Record<string, Response> = {
      "gate:refuses": rowsPage(blocked),
      "status:published": rowsPage([row()], 5),
      "status:scheduled": rowsPage([], 0),
      "status:draft": rowsPage([row({ status: "draft" })], 2),
    };
    return counts[query.get("filter") ?? ""] ?? rowsPage([row(), STUCK], 7);
  });
  return sent;
}

describe("the dashboard", () => {
  it("shows the blog at a glance under /admin with one main heading", async () => {
    signedIn();
    blog();
    renderAt("/");
    const heading = await screen.findByRole("heading", { level: 1, name: en.dashboard.title });
    expect(heading).toBeVisible();
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(screen.getByText(en.dashboard.calm)).toBeVisible();
    expect(screen.getByText(en.dashboard.live(5)).closest("li")).toHaveTextContent("5");
    expect(screen.getByText(en.dashboard.drafts(2)).closest("li")).toHaveTextContent("2");
    expect(screen.getByText(en.dashboard.blocked(0)).closest("li")).toHaveTextContent("0");
    expect(screen.getByRole("navigation", { name: en.nav.label })).toBeVisible();
    expect(screen.getByRole("link", { name: en.skip })).toHaveAttribute("href", "#main");
    expect(await axeViolations()).toEqual([]);
  });

  it("lists the latest changes with who made them and when", async () => {
    signedIn();
    const sent = blog();
    renderAt("/");
    const table = await screen.findByRole("table");
    const first = within(table).getAllByRole("row")[1];
    expect(first).toHaveTextContent("Why internal links matter");
    expect(first).toHaveTextContent(en.status.published);
    expect(first).toHaveTextContent("Asif");
    expect(first).toHaveTextContent("12 October 2026");
    const link = within(table).getByRole("link", { name: "Why internal links matter" });
    expect(link).toHaveAttribute("href", "/admin/posts/01JA");
    const recent = sent.map(listing).find((query) => query?.get("order") === "updated_at desc");
    expect(recent?.get("limit")).toBe("5");
  });

  it("makes blocked posts the one thing to look at", async () => {
    signedIn();
    blog([STUCK]);
    renderAt("/");
    const focus = await screen.findByRole("region", { name: en.dashboard.blockedTitle(1) });
    expect(within(focus).getByRole("link", { name: STUCK.title })).toHaveAttribute(
      "href",
      "/admin/posts/01JB",
    );
    expect(screen.getByText(en.dashboard.blockedLede(1))).toBeVisible();
    expect(screen.queryByLabelText(en.start.label)).toBeNull();
    expect(await axeViolations()).toEqual([]);
  });

  it("explains a list the blog would not send", async () => {
    signedIn();
    serve(() =>
      json(403, {
        type: "https://khalid.dev/problems/scope",
        title: "This key cannot list posts.",
        status: 403,
        detail: "Listing posts needs posts:write.",
        fix: "Sign in with a staff token that has the editor role.",
      }),
    );
    renderAt("/");
    expect(await screen.findByRole("alert")).toHaveTextContent("This key cannot list posts.");
  });
});
