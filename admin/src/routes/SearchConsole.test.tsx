import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import { followUps, NOT_SET_UP, searchConsoleBlog } from "../../test/searchConsole";
import { searchConsoleWords } from "../strings/searchConsoleWords";

const words = searchConsoleWords;
const ROUTE = "GET /search-console/follow-ups/";

describe("the Search Console screen", () => {
  it("leads with the searches no heading answers, then each post's searches", async () => {
    signedIn();
    searchConsoleBlog();
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: words.title }));
    expect(await screen.findByRole("heading", { level: 1, name: words.title })).toBeVisible();
    const focus = await screen.findByRole("region", { name: words.open.title });
    expect(focus).toHaveTextContent(words.open.count(2, 3));
    expect(focus).toHaveTextContent(words.open.sentence("1 October 2026", "28 October 2026"));
    const links = screen.getByRole("table", { name: "Why internal links matter" });
    const open = within(links).getByRole("row", { name: /how many internal links/ });
    expect(open).toHaveTextContent(words.posts.noHeading);
    expect(open).toHaveTextContent("60");
    expect(open).toHaveTextContent("8.2");
    const answered = within(links).getByRole("row", { name: /how do search engines/ });
    expect(answered).toHaveTextContent('"Search engines use them to find and rank pages"');
    expect(screen.getByRole("link", { name: "Why internal links matter" })).toHaveAttribute(
      "href",
      "https://www.example.com/blog/why-internal-links-matter/",
    );
    expect(screen.getByRole("link", { name: words.title })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(await axeViolations()).toEqual([]);
  });

  it("says so when every long search has a heading, and when there are none", async () => {
    signedIn();
    const answered = followUps();
    for (const post of answered.posts) {
      for (const search of post.searches) {
        search.answered_by = "A heading";
      }
    }
    searchConsoleBlog({ [ROUTE]: () => json(200, answered) });
    const { unmount } = renderAt("/search-console");
    expect(await screen.findByText(words.open.allAnswered(3))).toBeVisible();
    expect(screen.queryByText(words.posts.noHeading)).toBeNull();
    unmount();

    searchConsoleBlog({ [ROUTE]: () => json(200, followUps({ posts: [] })) });
    renderAt("/search-console");
    const none = words.open.none(8, "1 October 2026", "28 October 2026");
    expect(await screen.findByText(none)).toBeVisible();
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("shows the API's words once when the blog is not connected", async () => {
    signedIn();
    searchConsoleBlog({ [ROUTE]: () => json(409, NOT_SET_UP) });
    renderAt("/search-console");
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(NOT_SET_UP.title);
    expect(alert).toHaveTextContent(NOT_SET_UP.fix);
    expect(screen.getAllByRole("alert")).toHaveLength(1);
    expect(screen.queryByRole("table")).toBeNull();
  });
});
