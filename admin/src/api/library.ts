import { createApi } from "./client";
import type { MediaItem } from "./pictures";

/** How many pictures the library list shows at a time. */
export const LIBRARY_PAGE = 24;

/** A page of library pictures, or what to tell the writer when the list failed. */
export type Listed = { pictures: MediaItem[]; more: boolean } | { failed: string };

const api = createApi();

/** `text` as a quoted value in a list filter, with its quotes and backslashes escaped. */
function quoted(text: string): string {
  return `'${text.replace(/[\\']/g, (found) => `\\${found}`)}'`;
}

/**
 * The list filter for images whose name or alt text holds `words`, or every
 * image when `words` is blank (decision E12).
 */
export function libraryFilter(words: string): string {
  const wanted = words.trim();
  if (wanted === "") {
    return "kind:image";
  }
  return `kind:image+(name:~${quoted(wanted)},alt:~${quoted(wanted)})`;
}

/** Page `page` of the library's pictures that match `words`, newest upload first. */
export async function listPictures(words: string, page: number, fallback: string): Promise<Listed> {
  try {
    const { data } = await api.GET("/api/admin/v1/media/", {
      params: {
        query: { filter: libraryFilter(words), limit: String(LIBRARY_PAGE), page: String(page) },
      },
    });
    if (data === undefined) {
      return { failed: fallback };
    }
    return { pictures: data.media, more: typeof data.meta.pagination.next === "number" };
  } catch {
    return { failed: fallback };
  }
}
