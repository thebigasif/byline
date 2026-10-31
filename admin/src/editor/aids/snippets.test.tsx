import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { writing } from "../../../test/editor";
import { axeViolations, json, serve } from "../../../test/render";
import { type Snippet, snippetSlug } from "../../api/snippets";

const SNIPPET: Snippet = {
  id: "en.affiliate-disclosure",
  slug: "affiliate-disclosure",
  language: "en",
  title: "Affiliate disclosure",
  markdown: "Some links here earn us a small fee.",
  updated_at: "2026-10-30T09:00:00Z",
};

const AUTHOR_BOX: Snippet = {
  ...SNIPPET,
  id: "en.author-box",
  slug: "author-box",
  title: "About the author",
};

const POST = "Some links here earn us a small fee.\n\nWhy links matter.";

/** The blog's snippets; a new one is answered with `created`. */
function snippetsBlog(created = 201) {
  return serve(async (request) => {
    if (request.method === "POST") {
      const body: { snippets: Snippet[] } = await request.clone().json();
      return json(created, created === 201 ? { snippets: body.snippets } : {});
    }
    return json(200, { snippets: [SNIPPET, AUTHOR_BOX] });
  });
}

/** Selects the first paragraph of POST and opens the snippet form. */
function savingFirstParagraph() {
  const view = writing(POST);
  act(() => void view.editor.commands.setTextSelection({ from: 1, to: 37 }));
  fireEvent.click(screen.getByRole("button", { name: "Save as a snippet" }));
  return view;
}

describe("a snippet's slug", () => {
  it("is the name in small letters, digits and hyphens", () => {
    expect(snippetSlug("Affiliate disclosure")).toBe("affiliate-disclosure");
    expect(snippetSlug("  Über uns! ")).toBe("uber-uns");
    expect(snippetSlug("!!")).toBe("");
  });
});

describe("placing a snippet", () => {
  it("picks one of the site's snippets by name", async () => {
    snippetsBlog();
    const { type, press, onChange } = writing();
    type("/snippet");
    press("Enter");
    expect(screen.getByRole("form", { name: "Snippet" })).toBeInTheDocument();
    const choice = screen.getByLabelText("Which snippet");
    await screen.findByRole("option", { name: "About the author" });
    expect(await axeViolations()).toEqual([]);
    fireEvent.change(choice, { target: { value: "author-box" } });
    fireEvent.click(screen.getByRole("button", { name: "Add the block" }));
    expect(onChange).toHaveBeenLastCalledWith(
      expect.stringContaining("::snippet{slug=author-box}"),
    );
  });
});

describe("saving blocks as a snippet", () => {
  it("saves the selected blocks and puts the snippet in their place", async () => {
    const fetch = snippetsBlog();
    const { onChange } = savingFirstParagraph();
    expect(screen.getByRole("form", { name: "Save as a snippet" })).toBeInTheDocument();
    expect(screen.getByLabelText("Name")).toHaveFocus();
    expect(await axeViolations()).toEqual([]);
    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "Affiliate disclosure" } });
    fireEvent.click(screen.getByRole("button", { name: "Save the snippet" }));
    await waitFor(() =>
      expect(onChange).toHaveBeenLastCalledWith(
        expect.stringContaining("::snippet{slug=affiliate-disclosure}"),
      ),
    );
    const saved: string = onChange.mock.lastCall?.[0] ?? "";
    expect(saved).not.toContain("earn us");
    expect(saved).toContain("Why links matter.");
    const sent = fetch.mock.calls[0]?.[0];
    expect(sent?.method).toBe("POST");
    const { slug, title, markdown } = SNIPPET;
    expect(await sent?.json()).toEqual({ snippets: [{ slug, title, markdown }] });
  });

  it("says so when the name is taken, and keeps the blocks", async () => {
    snippetsBlog(409);
    const { onChange } = savingFirstParagraph();
    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "Affiliate disclosure" } });
    fireEvent.click(screen.getByRole("button", { name: "Save the snippet" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("already has this name");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("asks for a name with a letter or digit before it sends anything", () => {
    const fetch = snippetsBlog();
    savingFirstParagraph();
    fireEvent.click(screen.getByRole("button", { name: "Save the snippet" }));
    expect(screen.getByRole("alert")).toHaveTextContent("The snippet has no name.");
    expect(fetch).not.toHaveBeenCalled();
  });
});
