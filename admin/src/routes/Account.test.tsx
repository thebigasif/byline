import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { adminBlog } from "../../test/blog";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import { saveCsrf } from "../auth/session";
import { en } from "../strings/en";

const LENA = { account: { id: 1, email: "lena@example.com", name: "Lena", role: "editor" } };

describe("your account", () => {
  it("opens from the header and sends passkeys to their own screen", async () => {
    saveCsrf("csrf-test");
    const sent = adminBlog({ "GET /session/": () => json(200, { ...LENA, csrf: "csrf-test" }) });
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.account.nav }));
    expect(await screen.findByText(en.account.lede("Lena", "lena@example.com"))).toBeVisible();
    const passkeys = screen.getByRole("link", { name: en.account.passkey.link });
    expect(passkeys).toHaveAttribute("href", "/admin/passkeys");
    const signed = sent.every((request) => request.headers.get("x-khalid-csrf") === "csrf-test");
    expect(signed).toBe(true);
    expect(await axeViolations()).toEqual([]);
  });

  it("sets a first password, sending no current one", async () => {
    saveCsrf("csrf-test");
    const changes: unknown[] = [];
    adminBlog({
      "GET /session/": () => json(200, { ...LENA, csrf: "csrf-test" }),
      "PUT /session/password/": async (request) => {
        changes.push(await request.json());
        return new Response(null, { status: 204 });
      },
    });
    renderAt("/account");
    const words = en.account.password;
    await userEvent.type(await screen.findByLabelText(words.next), "a long first sentence");
    await userEvent.click(screen.getByRole("button", { name: words.submit }));
    expect(await screen.findByText(words.changed)).toBeVisible();
    expect(changes).toEqual([{ current: null, new: "a long first sentence" }]);
  });

  it("shows the API's words for a wrong current password", async () => {
    saveCsrf("csrf-test");
    const wrong = {
      type: "https://khalid.dev/problems/unauthorized",
      title: "The current password is wrong.",
      status: 401,
      detail: "Khalid changes a password only for someone who knows it.",
      fix: "Type your current password again.",
    };
    adminBlog({
      "GET /session/": () => json(200, { ...LENA, csrf: "csrf-test" }),
      "PUT /session/password/": () => json(401, wrong),
    });
    renderAt("/account");
    const words = en.account.password;
    await userEvent.type(await screen.findByLabelText(words.current), "not it");
    await userEvent.type(screen.getByLabelText(words.next), "a long new sentence");
    await userEvent.click(screen.getByRole("button", { name: words.submit }));
    expect(await screen.findByRole("alert")).toHaveTextContent(wrong.title);
  });

  it("signs out everywhere and forgets this tab's sign-in", async () => {
    saveCsrf("csrf-test");
    const sent = adminBlog({
      "GET /session/": () => json(200, { ...LENA, csrf: "csrf-test" }),
      "DELETE /sessions/": () => json(200, { ended: 3 }),
    });
    renderAt("/account");
    const words = en.account.everywhere;
    await userEvent.click(await screen.findByRole("button", { name: words.submit }));
    expect(await screen.findByRole("heading", { level: 1, name: en.signIn.title })).toBeVisible();
    expect(sent.some((request) => request.method === "DELETE")).toBe(true);
    expect(sessionStorage.getItem("khalid-admin-csrf")).toBeNull();
  });

  it("explains that a staff token has no account", async () => {
    signedIn();
    adminBlog({ "GET /session/": () => json(401, {}) });
    renderAt("/account");
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.noSession.what);
    expect(screen.queryByRole("link", { name: en.account.passkey.link })).toBeNull();
  });
});
