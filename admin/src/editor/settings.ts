import type { AdminPost, PostEdit } from "../api/posts";
import type { components } from "../api/schema";
import { localInput } from "./schedule";

/** Where a post stands in review. */
export type ReviewState = components["schemas"]["ReviewState"];
/** New values for some of a post's settings. */
export type SettingsEdit = components["schemas"]["SettingsEdit"];
type ShareCard = components["schemas"]["ShareCard"];

/** Every review state, in the order a post moves through them. */
export const REVIEW_STATES: readonly ReviewState[] = ["draft", "in_review", "ready"];

/**
 * A post's settings as the fields hold them: text, empty for none.
 * Co-authors are comma-separated, old addresses one per line, and
 * `unpublishAt` is a local date and time.
 */
export type SettingsDraft = {
  slug: string;
  author: string;
  coAuthors: string;
  date: string;
  excerpt: string;
  reviewState: ReviewState;
  imageSrc: string;
  imageAlt: string;
  imageCaption: string;
  canonical: string;
  robots: string;
  aliases: string;
  unpublishAt: string;
  xTitle: string;
  xDescription: string;
  xImage: string;
  facebookTitle: string;
  facebookDescription: string;
  facebookImage: string;
  seoTitle: string;
  focusKeyword: string;
  schema: string;
};

/** The settings whose field is free text. */
export type TextSetting = Exclude<keyof SettingsDraft, "reviewState">;

const IMAGE = ["imageSrc", "imageAlt", "imageCaption"] as const;
const CARDS = ["xTitle", "xDescription", "xImage"] as const;
const FACEBOOK = ["facebookTitle", "facebookDescription", "facebookImage"] as const;
const ONE_BY_ONE = ["slug", "author", "coAuthors", "date", "excerpt", "reviewState"] as const;
const OTHERS = ["canonical", "robots", "aliases", "unpublishAt"] as const;
const SEARCH = ["seoTitle", "focusKeyword", "schema"] as const;
const KEYS = [...ONE_BY_ONE, ...OTHERS, ...IMAGE, ...CARDS, ...FACEBOOK, ...SEARCH];

/** The panel's fields for `post`. */
export function settingsDraft(post: AdminPost): SettingsDraft {
  const { settings } = post;
  const { x, facebook } = settings.share_cards;
  return {
    slug: post.slug,
    author: post.author ?? "",
    coAuthors: settings.co_authors.join(", "),
    date: settings.date ?? "",
    excerpt: settings.excerpt ?? "",
    reviewState: settings.review_state,
    imageSrc: settings.feature_image?.src ?? "",
    imageAlt: settings.feature_image?.alt ?? "",
    imageCaption: settings.feature_image?.caption ?? "",
    canonical: settings.canonical ?? "",
    robots: settings.robots ?? "",
    aliases: settings.aliases.join("\n"),
    unpublishAt: post.unpublish_at ? localInput(post.unpublish_at) : "",
    xTitle: x?.title ?? "",
    xDescription: x?.description ?? "",
    xImage: x?.image ?? "",
    facebookTitle: facebook?.title ?? "",
    facebookDescription: facebook?.description ?? "",
    facebookImage: facebook?.image ?? "",
    seoTitle: settings.seo_title ?? "",
    focusKeyword: settings.focus_keyword ?? "",
    schema: settings.schema ?? "",
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

function list(text: string, separator: string): string[] {
  return text
    .split(separator)
    .map((each) => each.trim())
    .filter((each) => each !== "");
}

// A card from its three fields, or null when all are empty.
function card(title: string, description: string, image: string): ShareCard | null {
  const made = { title: orNull(title), description: orNull(description), image: orNull(image) };
  return made.title === null && made.description === null && made.image === null ? null : made;
}

// The local time as an instant with its offset; null for an empty or broken one.
function instant(local: string): string | null {
  const at = new Date(local);
  return local === "" || Number.isNaN(at.getTime()) ? null : at.toISOString();
}

function changes(saved: SettingsDraft, now: SettingsDraft, keys: readonly TextSetting[]) {
  return keys.some((key) => now[key] !== saved[key]);
}

function settingsChanged(saved: SettingsDraft, now: SettingsDraft): SettingsEdit {
  const settings: SettingsEdit = {};
  const changed = (...keys: TextSetting[]) => changes(saved, now, keys);
  if (changed("coAuthors")) {
    settings.co_authors = list(now.coAuthors, ",");
  }
  if (changed("date")) {
    settings.date = orNull(now.date);
  }
  if (changed("excerpt")) {
    settings.excerpt = orNull(now.excerpt);
  }
  if (now.reviewState !== saved.reviewState) {
    settings.review_state = now.reviewState;
  }
  if (changed(...IMAGE)) {
    const src = orNull(now.imageSrc);
    const caption = orNull(now.imageCaption);
    settings.feature_image = src === null ? null : { src, alt: now.imageAlt.trim(), caption };
  }
  if (changed(...CARDS, ...FACEBOOK)) {
    const x = card(now.xTitle, now.xDescription, now.xImage);
    const facebook = card(now.facebookTitle, now.facebookDescription, now.facebookImage);
    settings.share_cards = { x, facebook };
  }
  if (changed("aliases")) {
    settings.aliases = list(now.aliases, "\n");
  }
  if (changed("canonical")) {
    settings.canonical = orNull(now.canonical);
  }
  if (changed("robots")) {
    settings.robots = orNull(now.robots);
  }
  if (changed("seoTitle")) {
    settings.seo_title = orNull(now.seoTitle);
  }
  if (changed("focusKeyword")) {
    settings.focus_keyword = orNull(now.focusKeyword);
  }
  if (changed("schema")) {
    settings.schema = orNull(now.schema);
  }
  return settings;
}

/** The fields of an edit that change settings: only those that differ from `saved`. */
export function settingsEdit(saved: SettingsDraft, now: SettingsDraft): Partial<PostEdit> {
  const edit: Partial<PostEdit> = {};
  if (now.slug !== saved.slug) {
    edit.slug = now.slug.trim();
  }
  if (now.author !== saved.author) {
    edit.author = orNull(now.author);
  }
  if (now.unpublishAt !== saved.unpublishAt) {
    edit.unpublish_at = instant(now.unpublishAt);
  }
  const settings = settingsChanged(saved, now);
  if (Object.keys(settings).length > 0) {
    edit.settings = settings;
  }
  return edit;
}
