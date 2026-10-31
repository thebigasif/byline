import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { adminBlog, noContent } from "../../test/blog";
import { axeViolations, json, renderAt, serverAnswers } from "../../test/render";
import { en } from "../strings/en";

const LENA = { account: { id: 1, email: "lena@example.com", name: "Lena", role: "editor" } };

describe("signing in", () => {
  it("sends someone who is signed out to the sign-in screen", async () => {
    renderAt("/");
    expect(await screen.findByRole("heading", { level: 1, name: en.signIn.title })).toBeVisible();
    expect(await axeViolations()).toEqual([]);
  });

  it("keeps an accepted token for the tab and opens the admin", async () => {
    serverAnswers(404);
    renderAt("/sign-in");
    const field = await screen.findByLabelText(en.signIn.label);
    expect(field).toHaveAttribute("type", "password");
    await userEvent.type(field, "ks_0123_abcd");
    await userEvent.click(screen.getByRole("button", { name: en.signIn.submit }));
    const heading = await screen.findByRole("heading", { level: 1, name: en.dashboard.title });
    expect(heading).toBeVisible();
    expect(sessionStorage.getItem("khalid-admin-key")).toBe("ks_0123_abcd");
    expect(await axeViolations()).toEqual([]);
  });

  it("explains a refused token and keeps nothing", async () => {
    serverAnswers(401);
    renderAt("/sign-in");
    await userEvent.type(await screen.findByLabelText(en.signIn.label), "wrong");
    await userEvent.click(screen.getByRole("button", { name: en.signIn.submit }));
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.rejected.what);
    expect(sessionStorage.getItem("khalid-admin-key")).toBeNull();
  });

  it("signs in with an email, then signs each request with the session's CSRF token", async () => {
    const sent = adminBlog({ "POST /session/": () => json(200, { ...LENA, csrf: "csrf-test" }) });
    renderAt("/sign-in");
    await userEvent.type(await screen.findByLabelText(en.signIn.email.address), "lena@example.com");
    await userEvent.type(screen.getByLabelText(en.signIn.email.password), "correct horse");
    await userEvent.click(screen.getByRole("button", { name: en.signIn.email.submit }));
    const heading = await screen.findByRole("heading", { level: 1, name: en.dashboard.title });
    expect(heading).toBeVisible();
    expect(await sent[0]?.json()).toEqual({ email: "lena@example.com", password: "correct horse" });
    const later = sent.at(-1);
    expect(later?.headers.get("x-khalid-csrf")).toBe("csrf-test");
    expect(later?.headers.get("Authorization")).toBeNull();
    expect(sessionStorage.getItem("khalid-admin-key")).toBeNull();
  });

  it("shows the API's words for a wrong password, and keeps nothing", async () => {
    const wrong = {
      type: "https://khalid.dev/problems/unauthorized",
      title: "The email and password do not match.",
      status: 401,
      detail: "Khalid found no account with both.",
      fix: "Check both and try again.",
    };
    adminBlog({ "POST /session/": () => json(401, wrong) });
    renderAt("/sign-in");
    await userEvent.type(await screen.findByLabelText(en.signIn.email.address), "lena@example.com");
    await userEvent.type(screen.getByLabelText(en.signIn.email.password), "wrong");
    await userEvent.click(screen.getByRole("button", { name: en.signIn.email.submit }));
    expect(await screen.findByRole("alert")).toHaveTextContent(wrong.title);
    expect(sessionStorage.getItem("khalid-admin-csrf")).toBeNull();
  });

  it("signs in with the token after the # of a sign-in link, and opens the account", async () => {
    const sent = adminBlog({
      "POST /session/link/redeem/": () => json(200, { ...LENA, csrf: "csrf-link" }),
      "GET /session/": () => json(200, { ...LENA, csrf: "csrf-link" }),
    });
    renderAt("/sign-in/link#token=link_test");
    const heading = await screen.findByRole("heading", { level: 1, name: en.account.title });
    expect(heading).toBeVisible();
    expect(await sent[0]?.json()).toEqual({ token: "link_test" });
    expect(sessionStorage.getItem("khalid-admin-csrf")).toBe("csrf-link");
  });

  it("explains a sign-in link that no longer works", async () => {
    adminBlog({ "POST /session/link/redeem/": () => json(401, {}) });
    renderAt("/sign-in/link#token=used");
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.badLink.what);
    expect(screen.getByRole("link", { name: en.signIn.link.back })).toBeVisible();
    expect(await axeViolations()).toEqual([]);
  });

  it("ends an email sign-in on the server when signing out", async () => {
    sessionStorage.setItem("khalid-admin-csrf", "csrf-test");
    const sent = adminBlog({ "DELETE /session/": noContent });
    renderAt("/");
    await userEvent.click(await screen.findByRole("button", { name: en.signOut }));
    expect(await screen.findByRole("heading", { level: 1, name: en.signIn.title })).toBeVisible();
    expect(sent.some((request) => request.method === "DELETE")).toBe(true);
    expect(sessionStorage.getItem("khalid-admin-csrf")).toBeNull();
  });

  it("signs out and forgets the token", async () => {
    sessionStorage.setItem("khalid-admin-key", "ks_0123_abcd");
    renderAt("/");
    await userEvent.click(await screen.findByRole("button", { name: en.signOut }));
    expect(await screen.findByRole("heading", { level: 1, name: en.signIn.title })).toBeVisible();
    expect(sessionStorage.getItem("khalid-admin-key")).toBeNull();
  });
});
