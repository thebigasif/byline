import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { writing } from "../../../test/editor";
import { serve, serverAnswers } from "../../../test/render";
import { listing, row, rowsPage } from "../../../test/rows";
import { searchFilter } from "../../api/postSearch";

/** Answers every post search with `rows`, and keeps the filters it was sent. */
function blogWith(rows = [row()]) {
  const filters: (string | null)[] = [];
  serve((request) => {
    filters.push(listing(request)?.get("filter") ?? null);
    return rowsPage(rows);
  });
  return filters;
}

describe("finding a post by its words", () => {
  it("looks in the title and the slug", () => {
    expect(searchFilter(" internal ")).toBe("title:~'internal',slug:~'internal'");
  });

  it("quotes the writer's words for the list grammar", () => {
    expect(searchFilter("it's")).toBe("title:~'it\\'s',slug:~'it\\'s'");
  });

  it("lists the latest posts before any word is typed", () => {
    expect(searchFilter("")).toBeUndefined();
  });
});

describe("mentions and wikilinks", () => {
  it("mention a post with its title", async () => {
    const filters = blogWith();
    const { type, press, onChange } = writing();
    type("Read @inter");
    expect(await screen.findByRole("button", { name: /Why internal links matter/ })).toBeVisible();
    expect(screen.getByRole("region", { name: "Mention a post" })).toBeInTheDocument();
    expect(filters).toContain("title:~'inter',slug:~'inter'");
    press("Enter");
    expect(onChange).toHaveBeenLastCalledWith(
      "Read [[why-internal-links-matter|Why internal links matter]]\n",
    );
  });

  it("write a bare wikilink after two brackets", async () => {
    blogWith();
    const { type, press, onChange } = writing();
    type("See [[why");
    await screen.findByRole("button", { name: /Why internal links matter/ });
    press("Enter");
    expect(onChange).toHaveBeenLastCalledWith("See [[why-internal-links-matter]]\n");
  });

  it("say so when no post matches", async () => {
    blogWith([]);
    const { type } = writing();
    type("@zzz");
    expect(await screen.findByText(/No post has those words/)).toBeInTheDocument();
  });

  it("say so when the blog does not answer", async () => {
    serverAnswers(500);
    const { type } = writing();
    type("@inter");
    expect(await screen.findByText(/The blog did not answer/)).toBeInTheDocument();
  });
});
