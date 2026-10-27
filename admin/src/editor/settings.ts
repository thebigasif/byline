import type { AdminPost, PostEdit } from "../api/posts";
import type { components } from "../api/schema";

/** Where a post stands in review. */
export type ReviewState = components["schemas"]["ReviewState"];
/** New values for some of a post's settings. */
export type SettingsEdit = components["schemas"]["SettingsEdit"];

/** Every review state, in the order a post moves through them. */
export const REVIEW_STATES: readonly ReviewState[] = ["draft", "in_review", "ready"];

/** A post's settings as the fields hold them: text, empty for none, co-authors comma-separated. */
export type SettingsDraft = {
  slug: string;
  author: string;
  coAuthors: string;
  date: string;
  excerpt: string;
  reviewState: ReviewState;
};

/** The settings whose field is free text. */
export type TextSetting = Exclude<keyof SettingsDraft, "reviewState">;

const KEYS = ["slug", "author", "coAuthors", "date", "excerpt", "reviewState"] as const;

/** The panel's fields for `post`. */
export function settingsDraft(post: AdminPost): SettingsDraft {
  return {
    slug: post.slug,
    author: post.author ?? "",
    coAuthors: post.settings.co_authors.join(", "),
    date: post.settings.date ?? "",
    excerpt: post.settings.excerpt ?? "",
    reviewState: post.settings.review_state,
  };
}

/** `draft` with one setting changed. */
export function withSetting<K extends keyof SettingsDraft>(
  draft: SettingsDraft,
  key: K,
  value: SettingsDraft[K],
): SettingsDraft {
  const next = { ...draft };
  next[key] = value;
  return next;
}

/** Whether two drafts hold the same settings. */
export function sameSettings(a: SettingsDraft, b: SettingsDraft): boolean {
  return KEYS.every((key) => a[key] === b[key]);
}

// Trimmed text, or null when nothing is left, which clears the setting.
function orNull(text: string): string | null {
  const trimmed = text.trim();
  return trimmed === "" ? null : trimmed;
}

function list(text: string): string[] {
  return text
    .split(",")
    .map((each) => each.trim())
    .filter((each) => each !== "");
}

/** The fields of an edit that change settings: only those that differ from `saved`. */
export function settingsEdit(saved: SettingsDraft, now: SettingsDraft): Partial<PostEdit> {
  const edit: Partial<PostEdit> = {};
  const settings: SettingsEdit = {};
  if (now.slug !== saved.slug) {
    edit.slug = now.slug.trim();
  }
  if (now.author !== saved.author) {
    edit.author = orNull(now.author);
  }
  if (now.coAuthors !== saved.coAuthors) {
    settings.co_authors = list(now.coAuthors);
  }
  if (now.date !== saved.date) {
    settings.date = orNull(now.date);
  }
  if (now.excerpt !== saved.excerpt) {
    settings.excerpt = orNull(now.excerpt);
  }
  if (now.reviewState !== saved.reviewState) {
    settings.review_state = now.reviewState;
  }
  if (Object.keys(settings).length > 0) {
    edit.settings = settings;
  }
  return edit;
}
