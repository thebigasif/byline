import type { MediaQuery } from "../api/media";

/** The Media screen's address: which files to show and the page, each left out by default. */
export type MediaSearch = { show?: "no-alt"; page?: number };

/** Reads the Media screen's address, dropping anything it does not know. */
export function mediaSearch(search: Record<string, unknown>): MediaSearch {
  const page = Number(search.page);
  return {
    show: search.show === "no-alt" ? "no-alt" : undefined,
    page: Number.isInteger(page) && page > 1 ? page : undefined,
  };
}

/** The list query for the Media screen's address. */
export function mediaQuery(search: MediaSearch): MediaQuery {
  return { missingAlt: search.show === "no-alt", page: search.page };
}
