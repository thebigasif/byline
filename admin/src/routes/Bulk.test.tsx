import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { redirectsBlog } from "../../test/redirects";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import { row, rowsPage } from "../../test/rows";
import { en } from "../strings/en";

const LINKS = row();
const CLUSTER = row({
  id: "01JB",
  slug: "planning-a-topic-cluster",
  title: "Planning a topic cluster in an afternoon",
  author: "lena-vogt",
});

/** The problem a post someone else changed meanwhile gets. */
const STALE = {
  type: "https://khalid.dev/problems/conflict",
  title: "Someone else changed this first",
  status: 409,
  detail: "Lena changed this post at 2026-10-31 09:30 UTC, after the copy you edited.",
  fix: "Fetch the post again, make your change to it, and send its new updated_at.",
};

/** Ticks both posts on the page. */
async function tickBoth() {
  await userEvent.click(await screen.findByRole("checkbox", { name: en.bulk.tick(LINKS.title) }));
  await userEvent.click(screen.getByRole("checkbox", { name: en.bulk.tick(CLUSTER.title) }));
}

describe("changing many posts at once", () => {
  it("sets the author of each ticked post and says which was left", async () => {
    signedIn();
    const sent = redirectsBlog({
      "GET /posts/": () => rowsPage([LINKS, CLUSTER]),
      "PUT /posts/01JA/": () => json(200, { posts: [{ id: LINKS.id }] }),
      "PUT /posts/01JB/": () => json(409, STALE),
    });
    renderAt("/posts");
    await tickBoth();
    const form = screen.getByRole("form", { name: en.bulk.label });
    expect(within(form).getByText(en.bulk.ticked(2))).toBeVisible();
    expect(await axeViolations()).toEqual([]);
    await userEvent.selectOptions(within(form).getByLabelText(en.bulk.newAuthor), "lena-vogt");
    await userEvent.click(within(form).getByRole("button", { name: en.bulk.submit(2) }));
    expect(await screen.findByText(en.bulk.done(1))).toBeVisible();
    expect(screen.getByText(en.bulk.left(CLUSTER.title, STALE.title))).toBeVisible();
    const put = sent.find((request) => request.method === "PUT" && request.url.includes("01JA"));
    expect(await put?.json()).toEqual({
      posts: [{ updated_at: LINKS.updated_at, author: "lena-vogt" }],
    });
    expect(screen.queryByRole("form", { name: en.bulk.label })).toBeNull();
  });

  it("moves the ticked posts to the trash from the copy each row was read from", async () => {
    signedIn();
    const sent = redirectsBlog({
      "GET /posts/": () => rowsPage([LINKS, CLUSTER]),
      "DELETE /posts/01JA/": () => new Response(null, { status: 204 }),
      "DELETE /posts/01JB/": () => new Response(null, { status: 204 }),
    });
    renderAt("/posts");
    await tickBoth();
    const form = screen.getByRole("form", { name: en.bulk.label });
    await userEvent.selectOptions(within(form).getByLabelText(en.bulk.action), "trash");
    expect(within(form).queryByLabelText(en.bulk.newAuthor)).toBeNull();
    await userEvent.click(within(form).getByRole("button", { name: en.bulk.submit(2) }));
    expect(await screen.findByText(en.bulk.done(2))).toBeVisible();
    const deletes = sent.filter((request) => request.method === "DELETE");
    expect(deletes).toHaveLength(2);
    const seen = new URL(deletes[0]?.url ?? "").searchParams.get("updated_at");
    expect(seen).toBe(LINKS.updated_at);
  });
});
