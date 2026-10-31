import { useQuery } from "@tanstack/react-query";
import { createApi } from "./client";
import type { components } from "./schema";

/** A photo from Unsplash or a GIF from GIPHY, found for a post. */
export type StockPicture = components["schemas"]["StockPicture"];

/** Where to look: `unsplash` or `gifs`. */
export type StockSource = components["schemas"]["StockSource"];

/** Which searches the owner turned on. */
export type Sources = { unsplash: boolean; gifs: boolean };

/** The pictures a search found, or what to tell the writer when it failed. */
export type Found = { pictures: StockPicture[] } | { failed: string };

const api = createApi();

/** Words for a failed search: the blog's own words when it sent any. */
function failure(error: unknown, fallback: string): string {
  if (typeof error === "object" && error !== null && "detail" in error) {
    const fix = "fix" in error && typeof error.fix === "string" ? ` ${error.fix}` : "";
    return `${String(error.detail)}${fix}`;
  }
  return fallback;
}

/** Which searches are on; neither when the blog cannot say. */
export async function pictureSources(): Promise<Sources> {
  try {
    const { data } = await api.GET("/api/admin/v1/media/search/");
    return { unsplash: data?.unsplash === true, gifs: data?.gifs === true };
  } catch {
    return { unsplash: false, gifs: false };
  }
}

/** A search as Settings shows it: off, ready to ask, or on without its key (decision E15). */
export type SearchState = "off" | "ready" | "missing";

/** The state of each search. */
export type SearchStates = Record<StockSource, SearchState>;

/** The state of a search that is `on` and has its `key` or not. */
export function stateOf(on: boolean, key: boolean): SearchState {
  if (!on) {
    return "off";
  }
  return key ? "ready" : "missing";
}

/** The state of each search, or `null` when the blog cannot say. */
export async function searchStates(): Promise<SearchStates | null> {
  try {
    const { data } = await api.GET("/api/admin/v1/media/search/");
    if (data === undefined) {
      return null;
    }
    return {
      unsplash: stateOf(data.unsplash, data.keys.unsplash),
      gifs: stateOf(data.gifs, data.keys.gifs),
    };
  } catch {
    return null;
  }
}

/** Which searches are on, once the blog has said. */
export function usePictureSources(): Sources | undefined {
  return useQuery({ queryKey: ["picture-sources"], queryFn: pictureSources }).data;
}

/** Page 1 of the pictures `source` has for `words`. */
export async function searchPictures(
  source: StockSource,
  words: string,
  fallback: string,
): Promise<Found> {
  try {
    const { data, error } = await api.GET("/api/admin/v1/media/search/", {
      params: { query: { source, words, page: 1 } },
    });
    return data === undefined ? { failed: failure(error, fallback) } : { pictures: data.pictures };
  } catch {
    return { failed: fallback };
  }
}

/** A file in the media library, as the Admin API sends it. */
export type MediaItem = components["schemas"]["AdminMedia"];

/** A turn and a crop box for a copy of a picture. */
export type PictureEdit = components["schemas"]["PictureEditBody"];

/** The library item `id`, or `null` when the blog cannot say. */
export async function readPicture(id: string): Promise<MediaItem | null> {
  try {
    const { data } = await api.GET("/api/admin/v1/media/{id}/", { params: { path: { id } } });
    return data?.media[0] ?? null;
  } catch {
    return null;
  }
}

/** The library item `id`, once the blog has said. */
export function usePicture(id: string): MediaItem | null | undefined {
  return useQuery({ queryKey: ["media", id], queryFn: () => readPicture(id) }).data;
}

/** Makes a turned or cropped copy of picture `id`; gives back where the copy lives. */
export async function copyPicture(
  id: string,
  edit: PictureEdit,
  fallback: string,
): Promise<{ url: string } | { failed: string }> {
  try {
    const { data, error } = await api.POST("/api/admin/v1/media/{id}/copies/", {
      params: { path: { id } },
      body: edit,
    });
    const copy = data?.media[0];
    return copy === undefined ? { failed: failure(error, fallback) } : { url: copy.url };
  } catch {
    return { failed: fallback };
  }
}

/** Tells Unsplash, through the blog, that photo `id` went into a post. */
export async function photoUsed(id: string): Promise<void> {
  try {
    await api.POST("/api/admin/v1/media/search/used/", { body: { id } });
  } catch {
    // The post has the photo either way; a missed ping only goes uncounted.
  }
}
