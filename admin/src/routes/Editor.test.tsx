import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import type { AdminPost } from "../api/posts";
import { en } from "../strings/en";

const CALLOUT = ':::callout{tone="tip"}\nFeed the starter daily.\n:::\n';

function post(markdown: string): AdminPost {
  return {
    id: "01JB",
    slug: "planning-a-topic-cluster",
    title: "Planning a topic cluster",
    status: "draft",
    markdown,
    updated_at: "2026-10-27T10:42:00Z",
  };
}

describe("the editor", () => {
  it("opens a post by its identity with its title and markdown", async () => {
    signedIn();
    const fetch = serve(() => json(200, { posts: [post(CALLOUT)] }));
    renderAt("/posts/01JB");
    expect(await screen.findByLabelText(en.editor.title)).toHaveValue("Planning a topic cluster");
    expect(screen.getByRole("textbox", { name: en.editor.text })).toHaveValue(CALLOUT);
    expect(fetch.mock.calls[0]?.[0].url).toContain("/api/admin/v1/posts/01JB/");
    expect(await axeViolations()).toEqual([]);
  });

  it("says when the post does not exist", async () => {
    signedIn();
    serve(() => json(404, {}));
    renderAt("/posts/missing");
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
      en.problems.notFound.what,
    );
  });

  it("says when the blog cannot be reached", async () => {
    signedIn();
    serve(() => {
      throw new TypeError("Failed to fetch");
    });
    renderAt("/posts/01JB");
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
      en.problems.unreachable.what,
    );
  });
});
