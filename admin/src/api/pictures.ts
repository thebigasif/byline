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

/** Tells Unsplash, through the blog, that photo `id` went into a post. */
export async function photoUsed(id: string): Promise<void> {
  try {
    await api.POST("/api/admin/v1/media/search/used/", { body: { id } });
  } catch {
    // The post has the photo either way; a missed ping only goes uncounted.
  }
}
