import type { MediaItem } from "../src/api/media";
import { json } from "./render";

/** An image in the library with no alt text yet, unless `fields` say otherwise. */
export function item(fields: Partial<MediaItem> = {}): MediaItem {
  return {
    id: "3f2a9c1e0b57d0aa",
    name: "page-weight.png",
    folder: "guides",
    type: "image/png",
    kind: "image",
    bytes: 48213,
    width: 1600,
    height: 900,
    alt: "",
    caption: "",
    focal_point: { x: 500, y: 500 },
    url: "https://www.example.com/blog/media/library/3f2a9c1e0b57d0aa/page-weight.png",
    uploaded_at: "2026-10-05T08:00:00Z",
    uploaded_by: "asif",
    updated_at: "2026-10-05T08:00:00Z",
    updated_by: "asif",
    ...fields,
  };
}

/** A PDF in the top of the library, which needs no alt text. */
export function pdf(): MediaItem {
  return item({
    id: "9b1d44e07c2a6f13",
    name: "title-tag-checklist.pdf",
    folder: "",
    type: "application/pdf",
    kind: "file",
    bytes: 94208,
    width: null,
    height: null,
    url: "https://www.example.com/blog/media/library/9b1d44e07c2a6f13/title-tag-checklist.pdf",
  });
}

/** Answers a list request with `media` as page `page` of a library `total` long. */
export function mediaPage(media: MediaItem[], total = media.length, page = 1): Response {
  const limit = 30;
  const pages = Math.max(1, Math.ceil(total / limit));
  const pagination = {
    page,
    limit,
    pages,
    total,
    next: page < pages ? page + 1 : null,
    prev: page > 1 ? page - 1 : null,
  };
  return json(200, { media, meta: { pagination } });
}
