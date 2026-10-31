import type { AiVisitors, Uncited, Visitors } from "../src/api/analytics";
import type { Answers } from "./redirects";
import { json, serve } from "./render";

/** A week of page views ending 31 October 2026, as the Admin API sends it. */
export function visitors(fields: Partial<Visitors> = {}): Visitors {
  return {
    first: "2026-10-25",
    last: "2026-10-31",
    days: 7,
    views: 321,
    before: 287,
    arrivals: 259,
    sources: [
      { source: "direct", views: 78, before: 70 },
      { source: "search", views: 125, before: 108 },
      { source: "social", views: 16, before: 20 },
      { source: "ai", views: 30, before: 18 },
      { source: "other", views: 10, before: 9 },
      { source: "internal", views: 62, before: 62 },
    ],
    per_day: [
      { day: "2026-10-25", views: 49 },
      { day: "2026-10-26", views: 35 },
      { day: "2026-10-27", views: 63 },
      { day: "2026-10-28", views: 52 },
      { day: "2026-10-29", views: 0 },
      { day: "2026-10-30", views: 63 },
      { day: "2026-10-31", views: 59 },
    ],
    pages: [
      { path: "/blog/why-internal-links-matter/", views: 101 },
      { path: "/blog/how-to-write-a-title-tag/", views: 58 },
    ],
    ...fields,
  };
}

/** The readers AI assistants sent in the same week. */
export function aiVisitors(fields: Partial<AiVisitors> = {}): AiVisitors {
  return {
    first: "2026-10-25",
    last: "2026-10-31",
    days: 7,
    arrivals: 259,
    ai_views: 30,
    tools: [
      {
        tool: "chatgpt",
        name: "ChatGPT",
        views: 21,
        before: 12,
        pages: [{ path: "/blog/why-internal-links-matter/", views: 15 }],
      },
      { tool: "claude", name: "Claude", views: 0, before: 3, pages: [] },
    ],
    ...fields,
  };
}

/** The pages AI bots read in the same week with no reader from an AI assistant. */
export function uncited(fields: Partial<Uncited> = {}): Uncited {
  return {
    first: "2026-10-25",
    last: "2026-10-31",
    days: 7,
    pages: [
      {
        path: "/blog/what-a-sitemap-does/",
        reads: 37,
        bots: ["ClaudeBot", "GPTBot"],
        last_read: "2026-10-30T09:12:00Z",
      },
    ],
    ...fields,
  };
}

// What the Analytics screen reads when it opens, the week above unless a test says otherwise.
const WEEK: Answers = {
  "GET /analytics/visitors/": () => json(200, visitors()),
  "GET /analytics/ai-visitors/": () => json(200, aiVisitors()),
  "GET /analytics/uncited/": () => json(200, uncited()),
};

/** A blog that answers the analytics routes from `answers`, and the requests it was sent. */
export function analyticsBlog(answers: Answers = {}) {
  const sent: Request[] = [];
  const all = { ...WEEK, ...answers };
  serve((request) => {
    sent.push(request);
    const route = new URL(request.url).pathname.replace("/api/admin/v1", "");
    return all[`${request.method} ${route}`]?.(request) ?? json(404, {});
  });
  return sent;
}
