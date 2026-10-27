import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { axeViolations, renderAt, serverAnswers } from "../../test/render";
import { en } from "../strings/en";

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
    expect(await screen.findByRole("heading", { level: 1, name: en.home.title })).toBeVisible();
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

  it("signs out and forgets the token", async () => {
    sessionStorage.setItem("khalid-admin-key", "ks_0123_abcd");
    renderAt("/");
    await userEvent.click(await screen.findByRole("button", { name: en.signOut }));
    expect(await screen.findByRole("heading", { level: 1, name: en.signIn.title })).toBeVisible();
    expect(sessionStorage.getItem("khalid-admin-key")).toBeNull();
  });
});
