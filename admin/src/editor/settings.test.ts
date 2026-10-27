import { describe, expect, it } from "vitest";
import type { AdminPost } from "../api/posts";
import { localInput } from "./schedule";
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
    expect(settingsDraft(POST)).toMatchObject({
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

describe("more settings", () => {
  const SHOWN: AdminPost = {
    ...POST,
    unpublish_at: "2026-12-01T09:00:00Z",
    settings: {
      ...POST.settings,
      feature_image: { src: "/media/cluster.jpg", alt: "Index cards on a desk" },
      share_cards: { x: { title: "Plan a cluster in an afternoon" } },
      aliases: ["/2019/05/clusters/", "/clusters/"],
    },
  };

  it("holds the image, cards and old addresses as their fields show them", () => {
    expect(settingsDraft(SHOWN)).toMatchObject({
      imageSrc: "/media/cluster.jpg",
      imageAlt: "Index cards on a desk",
      imageCaption: "",
      xTitle: "Plan a cluster in an afternoon",
      facebookTitle: "",
      aliases: "/2019/05/clusters/\n/clusters/",
      unpublishAt: localInput("2026-12-01T09:00:00Z"),
    });
  });

  it("sends the whole image and both cards when one of their fields changed", () => {
    const saved = settingsDraft(SHOWN);
    let now = withSetting(saved, "imageCaption", "Planning on paper");
    now = withSetting(now, "facebookDescription", "One main post, several short ones.");
    now = withSetting(now, "aliases", "/clusters/\n\n");
    now = withSetting(now, "robots", "noindex");
    now = withSetting(now, "unpublishAt", "");
    expect(settingsEdit(saved, now)).toEqual({
      unpublish_at: null,
      settings: {
        feature_image: {
          src: "/media/cluster.jpg",
          alt: "Index cards on a desk",
          caption: "Planning on paper",
        },
        share_cards: {
          x: { title: "Plan a cluster in an afternoon", description: null, image: null },
          facebook: { title: null, description: "One main post, several short ones.", image: null },
        },
        aliases: ["/clusters/"],
        robots: "noindex",
      },
    });
  });

  it("clears the image when its address is emptied", () => {
    const saved = settingsDraft(SHOWN);
    const edit = settingsEdit(saved, withSetting(saved, "imageSrc", " "));
    expect(edit).toEqual({ settings: { feature_image: null } });
  });
});
