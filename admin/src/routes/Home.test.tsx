import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, renderAt, signedIn } from "../../test/render";
import { en } from "../strings/en";

describe("the admin shell", () => {
  it("shows the home screen under /admin with one main heading", async () => {
    signedIn();
    renderAt("/");
    expect(await screen.findByRole("heading", { level: 1, name: en.home.title })).toBeVisible();
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(screen.getByRole("navigation", { name: en.nav.label })).toBeVisible();
    expect(screen.getByRole("link", { name: en.skip })).toHaveAttribute("href", "#main");
    expect(await axeViolations()).toEqual([]);
  });
});
