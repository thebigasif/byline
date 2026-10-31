import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve, signedIn } from "../../test/render";
import type { PostCheck } from "../api/check";
import type { AdminPost } from "../api/posts";
import { en } from "../strings/en";

const POST: AdminPost = {
  id: "en.why-internal-links-matter",
  slug: "why-internal-links-matter",
  title: "Why internal links matter",
  status: "published",
  markdown: "Internal links help readers and search engines find the rest of your site.\n",
  updated_at: "2026-10-20T08:30:00Z",
  settings: {
    co_authors: [],
    featured: false,
    review_state: "ready",
    access: "public",
    password_protected: false,
    share_cards: {},
    aliases: [],
  },
};

const NO_SOURCE = {
  rule: "no-external-link",
  severity: "warn",
  locale: "en",
  slug: "why-internal-links-matter",
  subject: null,
  what: "This post links to no other website.",
  why: "Links to good sources show readers and Google where your facts come from.",
  fix: "Add a link to a trusted source that backs up a claim in the post.",
};

const CHECK: PostCheck = {
  id: POST.id,
  lint: {
    blocks: false,
    findings: [NO_SOURCE],
    score: { value: 95, lost: [{ rule: "no-external-link", points: 5 }] },
  },
  snippet: {
    title: "Why internal links matter | Field Notes on SEO",
    url: "https://www.example.com/blog/why-internal-links-matter/",
    description: "Internal links help readers and search engines find the rest of your site.",
  },
  schema: { "@context": "https://schema.org", "@graph": [{ "@type": "BlogPosting" }] },
};

const EVENT = '{"@type": "Event", "name": "Internal links clinic"}';

/** A blog holding the demo post, which answers its check with `check` and records edits. */
function blog(check: () => Response) {
  const edits: unknown[] = [];
  serve(async (request) => {
    if (request.url.endsWith("/check/")) {
      return check();
    }
    if (request.method === "PUT") {
      const body: { posts: unknown[] } = await request.json();
      edits.push(body.posts[0]);
    }
    return json(200, { posts: [POST] });
  });
  return edits;
}

describe("the editor's score and search result", () => {
  it("shows the score beside the checks, and the post as a Google result", async () => {
    signedIn();
    blog(() => json(200, { checks: [CHECK] }));
    renderAt(`/posts/${POST.id}`);
    expect(await screen.findByText(en.editor.score(95))).toBeVisible();
    expect(screen.getByText(NO_SOURCE.what)).toBeVisible();
    const result = screen.getByRole("region", { name: en.search.result });
    expect(result).toHaveTextContent("Why internal links matter | Field Notes on SEO");
    expect(result).toHaveTextContent("https://www.example.com › blog › why-internal-links-matter");
    expect(await axeViolations()).toEqual([]);
  });

  it("shows only the checks a publish brings back when the blog cannot check", async () => {
    signedIn();
    blog(() => json(404, { title: "This server does not check posts." }));
    renderAt(`/posts/${POST.id}`);
    expect(await screen.findByText(en.editor.noChecks)).toBeVisible();
    expect(screen.getByLabelText(en.search.focus)).toBeVisible();
    expect(screen.queryByRole("region", { name: en.search.result })).toBeNull();
  });
});

describe("the editor's search fields and schema", () => {
  it("lists the schema the page carries and saves the writer's own", async () => {
    signedIn();
    const edits = blog(() => json(200, { checks: [CHECK] }));
    renderAt(`/posts/${POST.id}`);
    // The schema panel starts folded, so its parts are found by their words.
    expect(await screen.findByText("BlogPosting")).toBeInTheDocument();
    const test = screen.getByText(en.schema.test).closest("a");
    expect(test).toHaveAttribute("href", expect.stringContaining("why-internal-links-matter"));
    const own = screen.getByLabelText(en.schema.own);
    fireEvent.change(own, { target: { value: EVENT } });
    const focus = screen.getByLabelText(en.search.focus);
    fireEvent.change(focus, { target: { value: "internal links" } });
    fireEvent.blur(focus);
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent(en.editor.saved));
    expect(edits).toEqual([
      {
        updated_at: POST.updated_at,
        title: POST.title,
        markdown: POST.markdown,
        settings: { focus_keyword: "internal links", schema: EVENT },
      },
    ]);
    expect(await axeViolations()).toEqual([]);
  });
});
