import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { analyticsBlog, visitors } from "../../test/analytics";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import { en } from "../strings/en";

const words = en.analytics;

describe("the Analytics screen", () => {
  it("shows the week's page views and where readers came from", async () => {
    signedIn();
    analyticsBlog();
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.analytics }));
    expect(await screen.findByRole("heading", { level: 1, name: words.title })).toBeVisible();
    const total = await screen.findByRole("region", { name: words.total(7) });
    expect(total).toHaveTextContent("321");
    expect(total).toHaveTextContent(words.rise(12, 7, "287"));
    expect(screen.getByText(words.chart.summary("0", "63", "27 October 2026"))).toBeVisible();
    const sources = screen.getByRole("table", { name: words.sources.title });
    const search = within(sources).getByRole("row", { name: /Search engines/ });
    expect(search).toHaveTextContent("125");
    expect(search).toHaveTextContent("108");
    expect(screen.getByRole("link", { name: en.nav.analytics })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(await axeViolations()).toEqual([]);
  });

  it("reads the page views again for the days picked", async () => {
    signedIn();
    const sent = analyticsBlog();
    renderAt("/analytics");
    await userEvent.selectOptions(await screen.findByLabelText(words.days), words.period(90));
    await userEvent.click(screen.getByRole("button", { name: words.show }));
    await waitFor(() => {
      const asked = sent.map((request) => new URL(request.url).searchParams.get("days"));
      expect(asked.filter((days) => days === "90")).toHaveLength(1);
    });
  });

  it("says plainly when nothing was counted", async () => {
    signedIn();
    const none = { views: 0, before: 0, arrivals: 0, pages: [] };
    const quiet = visitors({ ...none, per_day: [{ day: "2026-10-31", views: 0 }] });
    analyticsBlog({ "GET /analytics/visitors/": () => json(200, quiet) });
    renderAt("/analytics");
    expect(await screen.findByText(words.level(7))).toBeVisible();
    expect(screen.getByText(words.chart.none)).toBeVisible();
    expect(screen.queryByText(words.chart.table)).toBeNull();
  });

  it("shows the API's words once when the key may not read visitor data", async () => {
    signedIn();
    const lacks = {
      type: "https://khalid.dev/problems/forbidden",
      title: "This key cannot read visitor data.",
      status: 403,
      detail: "Analytics need the settings:write scope.",
      fix: "Sign in with an owner or admin staff token.",
    };
    analyticsBlog({ "GET /analytics/visitors/": () => json(403, lacks) });
    renderAt("/analytics");
    expect(await screen.findByRole("alert")).toHaveTextContent(lacks.title);
    expect(screen.getAllByRole("alert")).toHaveLength(1);
  });
});
