import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import type { AdminPost } from "../api/posts";
import type { Claim, PostPresence } from "../api/presence";
import { en } from "../strings/en";
import { presence as words } from "../strings/presence";

const DRAFT: AdminPost = {
  id: "01JB",
  slug: "why-internal-links-matter",
  title: "Why internal links matter",
  status: "draft",
  markdown: "Internal links help readers and Google find the rest of your site.\n",
  updated_at: "2026-10-31T10:42:00Z",
  settings: {
    co_authors: [],
    featured: false,
    review_state: "draft",
    access: "public",
    password_protected: false,
    share_cards: {},
    aliases: [],
  },
};

const MARIA = { who: "maria", since: "2026-10-31T10:40:00Z" };
const ME = { who: "asif", since: "2026-10-31T10:45:00Z" };

/** A blog where Maria holds DRAFT until this writer takes it over; records each claim. */
function heldByMaria() {
  const claims: Claim[] = [];
  let taken = false;
  serve(async (request) => {
    if (!request.url.endsWith("/presence/")) {
      return json(200, { posts: [DRAFT] });
    }
    if (request.method === "DELETE") {
      return new Response(null, { status: 204 });
    }
    const { claim }: { claim: Claim } = await request.json();
    claims.push(claim);
    taken ||= claim === "take_over";
    const seen: PostPresence = taken
      ? { holder: ME, yours: true, others: [MARIA] }
      : { holder: MARIA, yours: false, others: [] };
    return json(200, seen);
  });
  return claims;
}

describe("someone else editing the post", () => {
  it("locks the text, says who holds it and what taking over costs", async () => {
    signedIn();
    heldByMaria();
    renderAt("/posts/01JB");
    const held = await screen.findByRole("heading", { name: words.held("maria") });
    expect(held).toBeVisible();
    expect(screen.getByText(words.cost("maria"))).toBeVisible();
    expect(screen.getByLabelText(en.editor.title)).toHaveAttribute("readonly");
    const text = screen.getByRole("textbox", { name: en.editor.text });
    expect(text).toHaveAttribute("contenteditable", "false");
    expect(screen.queryByRole("button", { name: en.editor.publish })).toBeNull();
    expect(await axeViolations()).toEqual([]);
  });

  it("takes the post over, and then shows Maria is still reading", async () => {
    signedIn();
    const claims = heldByMaria();
    renderAt("/posts/01JB");
    fireEvent.click(await screen.findByRole("button", { name: words.takeOver }));
    expect(await screen.findByText(words.others(["maria"]))).toBeVisible();
    expect(claims).toEqual(["join", "take_over"]);
    expect(screen.queryByRole("heading", { name: words.held("maria") })).toBeNull();
    expect(screen.getByLabelText(en.editor.title)).not.toHaveAttribute("readonly");
    await waitFor(() =>
      expect(screen.getByRole("textbox", { name: en.editor.text })).toHaveAttribute(
        "contenteditable",
        "true",
      ),
    );
  });
});
