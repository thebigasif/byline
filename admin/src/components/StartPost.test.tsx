import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { json, renderAt, serve, signedIn } from "../../test/render";
import { listing, rowsPage } from "../../test/rows";
import { en } from "../strings/en";

const DRAFT = {
  id: "01JC",
  slug: "why-dough-sticks",
  title: "Why dough sticks",
  status: "draft",
  markdown: "",
  updated_at: "2026-10-27T11:00:00Z",
};

describe("starting a post", () => {
  it("starts a draft with one idempotency key and opens it in the editor", async () => {
    signedIn();
    const sent: Request[] = [];
    serve((request) => {
      if (listing(request) !== null) {
        return rowsPage([]);
      }
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
    serve((request) => {
      if (listing(request) !== null) {
        return rowsPage([]);
      }
      throw new TypeError("Failed to fetch");
    });
    renderAt("/");
    await userEvent.type(await screen.findByLabelText(en.start.label), "Why dough sticks");
    await userEvent.click(screen.getByRole("button", { name: en.start.submit }));
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.unreachable.what);
  });
});
