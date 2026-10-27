import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import { en } from "../strings/en";

const DRAFT = {
  id: "01JC",
  slug: "why-dough-sticks",
  title: "Why dough sticks",
  status: "draft",
  markdown: "",
  updated_at: "2026-10-27T11:00:00Z",
};

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

  it("starts a draft with one idempotency key and opens it in the editor", async () => {
    signedIn();
    const sent: Request[] = [];
    serve((request) => {
      sent.push(request);
      return json(request.method === "POST" ? 201 : 200, { posts: [DRAFT] });
    });
    const { router } = renderAt("/");
    await userEvent.type(await screen.findByLabelText(en.start.label), "Why dough sticks");
    await userEvent.click(screen.getByRole("button", { name: en.start.submit }));
    expect(await screen.findByLabelText(en.editor.title)).toHaveValue("Why dough sticks");
    expect(router.state.location.pathname).toContain("/posts/01JC");
    expect(sent[0]?.method).toBe("POST");
    expect(sent[0]?.headers.get("Idempotency-Key")).toMatch(/^[0-9a-f]{32}$/);
  });

  it("explains a draft the blog could not make", async () => {
    signedIn();
    serve(() => {
      throw new TypeError("Failed to fetch");
    });
    renderAt("/");
    await userEvent.type(await screen.findByLabelText(en.start.label), "Why dough sticks");
    await userEvent.click(screen.getByRole("button", { name: en.start.submit }));
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.unreachable.what);
  });
});
