import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { adminBlog } from "../../test/blog";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { StaffAccount } from "../api/staff";
import { en } from "../strings/en";

/** The team from the mockup; the addresses are made up. */
const TEAM: StaffAccount[] = [
  { id: 1, email: "asif@example.com", name: "Asif", role: "owner" },
  { id: 2, email: "lena@example.com", name: "Lena Vogt", role: "editor" },
];

const words = en.team.handover;

describe("the hand-over screen", () => {
  it("opens from Team, says what changes, and hands the site over once understood", async () => {
    signedIn();
    const sent = adminBlog({
      "GET /staff/": () => json(200, { staff: TEAM }),
      "POST /staff/2/owner/": () => json(200, { staff: [{ ...TEAM[1], role: "owner" }] }),
    });
    renderAt("/team");
    await userEvent.click(await screen.findByRole("link", { name: en.team.list.handOver }));
    expect(await screen.findByRole("heading", { level: 1, name: words.title })).toBeVisible();
    for (const consequence of words.consequences) {
      expect(screen.getByText(consequence)).toBeVisible();
    }
    expect(screen.getByLabelText(words.next)).toHaveDisplayValue("Lena Vogt, lena@example.com");
    const submit = screen.getByRole("button", { name: words.submit("Lena Vogt") });
    expect(submit).toBeDisabled();
    expect(await axeViolations()).toEqual([]);
    await userEvent.click(screen.getByLabelText(words.understand));
    await userEvent.click(submit);
    expect(await screen.findByText(words.done("Lena Vogt"))).toBeVisible();
    const posted = sent.filter((request) => request.method === "POST");
    expect(posted.map((request) => new URL(request.url).pathname)).toEqual([
      "/api/admin/v1/staff/2/owner/",
    ]);
  });

  it("shows the API's words when someone other than the owner tries", async () => {
    signedIn();
    const title = "Only the owner, signed in, may hand the site to someone else.";
    const refused = {
      type: "https://khalid.dev/problems/forbidden",
      title,
      status: 403,
      detail: "The API says why.",
      fix: "Ask the owner to do it.",
    };
    adminBlog({
      "GET /staff/": () => json(200, { staff: TEAM }),
      "POST /staff/2/owner/": () => json(403, refused),
    });
    renderAt("/team/owner");
    await userEvent.click(await screen.findByLabelText(words.understand));
    await userEvent.click(screen.getByRole("button", { name: words.submit("Lena Vogt") }));
    expect(await screen.findByText(title)).toBeVisible();
    expect(screen.queryByText(words.done("Lena Vogt"))).toBeNull();
  });

  it("asks for an invitation first when nobody else is on the team", async () => {
    signedIn();
    adminBlog({ "GET /staff/": () => json(200, { staff: [TEAM[0]] }) });
    renderAt("/team/owner");
    expect(await screen.findByText(words.nobody)).toBeVisible();
    expect(screen.queryByRole("button", { name: /the owner$/ })).toBeNull();
  });
});
