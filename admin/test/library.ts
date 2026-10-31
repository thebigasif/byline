import type { MediaItem } from "../src/api/pictures";

/** A library picture as the Admin API sends it. */
export function picture(changes: Partial<MediaItem> = {}): MediaItem {
  return {
    id: "0123456789abcdef",
    name: "lake.jpg",
    folder: "",
    type: "image/jpeg",
    kind: "image",
    bytes: 52_000,
    width: 1200,
    height: 800,
    alt: "A boat on a still lake",
    caption: "Lake Bled at dawn",
    focal_point: { x: 500, y: 500 },
    url: "https://blog.example/media/library/0123456789abcdef/lake.jpg",
    uploaded_at: "2026-10-30T09:00:00Z",
    uploaded_by: "asif",
    updated_at: "2026-10-30T09:00:00Z",
    updated_by: "asif",
    ...changes,
  };
}

/** A page of the library, with a next page when `next` is set. */
export function page(media: MediaItem[], next: number | null = null) {
  const total = media.length;
  return { media, meta: { pagination: { page: 1, limit: 24, pages: next ?? 1, total, next } } };
}
