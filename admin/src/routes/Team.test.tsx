import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { adminBlog, noContent } from "../../test/blog";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { StaffAccount } from "../api/staff";
import { en } from "../strings/en";

/** The team from the mockup; the addresses are made up. */
const TEAM: StaffAccount[] = [
  { id: 1, email: "asif@example.com", name: "Asif", role: "owner" },
  { id: 2, email: "lena@example.com", name: "Lena Vogt", role: "editor" },
];

const words = en.team;
const changeLena = words.list.changeLabel("Lena Vogt");

/** A problem as the API sends it, in plain words. */
function problem(status: number, title: string) {
  const type = status === 403 ? "forbidden" : "conflict";
  const detail = "The API says why.";
  return json(status, { type: `https://khalid.dev/problems/${type}`, title, status, detail });
}

describe("the Team screen", () => {
  it("lists the team, and the owner has no controls", async () => {
    signedIn();
    adminBlog({ "GET /staff/": () => json(200, { staff: TEAM }) });
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.team }));
    const table = await screen.findByRole("table", { name: words.list.title });
    const [, owner, lena] = within(table).getAllByRole("row");
    expect(owner).toHaveTextContent(words.list.owner);
    expect(within(owner ?? table).queryAllByRole("button")).toHaveLength(0);
    expect(lena).toHaveTextContent("lena@example.com");
    expect(await axeViolations()).toEqual([]);
  });

  it("invites someone as a contributor", async () => {
    signedIn();
    const asked: unknown[] = [];
    const mira = { id: 3, email: "mira@example.com", name: "Mira Hoffmann", role: "contributor" };
    adminBlog({
      "GET /staff/": () => json(200, { staff: TEAM }),
      "POST /staff/": async (request) => {
        asked.push(await request.json());
        return json(201, { staff: [mira] });
      },
    });
    renderAt("/team");
    await userEvent.type(await screen.findByLabelText(words.invite.name), "Mira Hoffmann");
    await userEvent.type(screen.getByLabelText(words.invite.email), "mira@example.com");
    // The editor's hint names contributors too, so match the label's start.
    await userEvent.click(screen.getByLabelText(new RegExp(`^${words.roles.contributor}`)));
    await userEvent.click(screen.getByRole("button", { name: words.invite.submit }));
    expect(await screen.findByText(words.invite.invited("Mira Hoffmann"))).toBeVisible();
    expect(asked).toEqual([
      { name: "Mira Hoffmann", email: "mira@example.com", role: "contributor" },
    ]);
    expect(await axeViolations()).toEqual([]);
  });

  it("shows the API's words when the email already has an account", async () => {
    signedIn();
    const taken = "An account already has this email.";
    adminBlog({
      "GET /staff/": () => json(200, { staff: TEAM }),
      "POST /staff/": () => problem(409, taken),
    });
    renderAt("/team");
    await userEvent.type(await screen.findByLabelText(words.invite.name), "Lena");
    await userEvent.type(screen.getByLabelText(words.invite.email), "lena@example.com");
    await userEvent.click(screen.getByRole("button", { name: words.invite.submit }));
    expect(await screen.findByRole("alert")).toHaveTextContent(taken);
    expect(screen.getByLabelText(words.invite.name)).toHaveValue("Lena");
  });

  it("changes a role and sends only the new role", async () => {
    signedIn();
    const asked: unknown[] = [];
    let team = [...TEAM];
    adminBlog({
      "GET /staff/": () => json(200, { staff: team }),
      "PUT /staff/2/": async (request) => {
        asked.push(await request.json());
        team = team.map((person) => (person.id === 2 ? { ...person, role: "admin" } : person));
        return json(200, { staff: team.filter((person) => person.id === 2) });
      },
    });
    renderAt("/team");
    const picker = await screen.findByLabelText(words.list.roleFor("Lena Vogt"));
    await userEvent.selectOptions(picker, "admin");
    await userEvent.click(screen.getByRole("button", { name: changeLena }));
    expect(await screen.findByText(words.list.changed("Lena Vogt", "Admin"))).toBeVisible();
    expect(asked).toEqual([{ role: "admin" }]);
    await waitFor(() =>
      expect(screen.getByLabelText(words.list.roleFor("Lena Vogt"))).toHaveValue("admin"),
    );
  });

  it("shows the API's words when the role may not make the change", async () => {
    signedIn();
    const refused = "This role cannot make that change to that account.";
    adminBlog({
      "GET /staff/": () => json(200, { staff: TEAM }),
      "PUT /staff/2/": () => problem(403, refused),
    });
    renderAt("/team");
    const picker = await screen.findByLabelText(words.list.roleFor("Lena Vogt"));
    await userEvent.selectOptions(picker, "admin");
    await userEvent.click(screen.getByRole("button", { name: changeLena }));
    expect(await screen.findByRole("alert")).toHaveTextContent(refused);
  });

  it("removes someone, and they leave the list", async () => {
    signedIn();
    let team = [...TEAM];
    const sent = adminBlog({
      "GET /staff/": () => json(200, { staff: team }),
      "DELETE /staff/2/": () => {
        team = team.filter((person) => person.id !== 2);
        return noContent();
      },
    });
    renderAt("/team");
    const remove = words.list.removeLabel("Lena Vogt");
    await userEvent.click(await screen.findByRole("button", { name: remove }));
    expect(await screen.findByText(words.list.removed("Lena Vogt"))).toBeVisible();
    await waitFor(() => expect(screen.queryByText("lena@example.com")).toBeNull());
    expect(sent.filter((request) => request.method === "DELETE")).toHaveLength(1);
  });
});
