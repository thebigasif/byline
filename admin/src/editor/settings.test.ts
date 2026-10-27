import { describe, expect, it } from "vitest";
import type { AdminPost } from "../api/posts";
import { sameSettings, settingsDraft, settingsEdit, withSetting } from "./settings";

const POST: AdminPost = {
  id: "01JB",
  slug: "planning-a-topic-cluster",
  title: "Planning a topic cluster",
  status: "draft",
  author: "asif",
  markdown: "",
  updated_at: "2026-10-27T10:42:00Z",
  settings: {
    co_authors: ["lena-vogt"],
    featured: false,
    review_state: "draft",
    share_cards: {},
    aliases: [],
    date: "2026-10-12",
  },
};

describe("the settings panel's draft", () => {
  it("holds each setting as its field shows it", () => {
    expect(settingsDraft(POST)).toEqual({
      slug: "planning-a-topic-cluster",
      author: "asif",
      coAuthors: "lena-vogt",
      date: "2026-10-12",
      excerpt: "",
      reviewState: "draft",
    });
  });

  it("sends nothing when nothing changed", () => {
    const draft = settingsDraft(POST);
    expect(settingsEdit(draft, draft)).toEqual({});
    expect(sameSettings(draft, { ...draft })).toBe(true);
  });

  it("sends only the settings that changed, trimmed, with empty as none", () => {
    const saved = settingsDraft(POST);
    let now = withSetting(saved, "slug", " plan-a-topic-cluster ");
    now = withSetting(now, "author", "");
    now = withSetting(now, "coAuthors", "lena-vogt, mira ,");
    now = withSetting(now, "excerpt", "One main post and several short ones.");
    now = withSetting(now, "reviewState", "in_review");
    expect(sameSettings(saved, now)).toBe(false);
    expect(settingsEdit(saved, now)).toEqual({
      slug: "plan-a-topic-cluster",
      author: null,
      settings: {
        co_authors: ["lena-vogt", "mira"],
        excerpt: "One main post and several short ones.",
        review_state: "in_review",
      },
    });
  });

  it("clears the date when its field is emptied", () => {
    const saved = settingsDraft(POST);
    const edit = settingsEdit(saved, withSetting(saved, "date", ""));
    expect(edit).toEqual({ settings: { date: null } });
  });
});
