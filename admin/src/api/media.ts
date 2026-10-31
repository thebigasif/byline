// The media library routes of the Admin API (KH-EDT-5).
import { readKey } from "../auth/session";
import { createApi } from "./client";
import { isProblem } from "./posts";
import type { Failure, Reply } from "./redirects";
import type { Pagination } from "./rows";
import type { components } from "./schema";

/** A file in the media library with the details kept beside it. */
export type MediaItem = components["schemas"]["AdminMedia"];
/** A change to an item's details, made on the copy `updated_at` names. */
export type MediaEdit = components["schemas"]["MediaEditBody"];

/** One page of the library and where it sits in the list. */
export type MediaPage = { media: MediaItem[]; pagination: Pagination };

/** Which page of the library to send, and whether only images with no alt text. */
export type MediaQuery = { missingAlt?: boolean; page?: number };

/** What an upload gave back: the item, and whether the library already had the file. */
export type Uploaded = { item: MediaItem; duplicate: boolean };

/** How many items a page of the library holds. */
export const MEDIA_PAGE = 30;

const api = createApi();

/** Whether a body is the `{ media: [...] }` wrapper the routes send. */
function hasItems(body: unknown): body is { media: MediaItem[] } {
  return typeof body === "object" && body !== null && "media" in body && Array.isArray(body.media);
}

/** The failure a call answered with; status 0 when the blog cannot be reached. */
function failed(error: unknown, response: Response | undefined): Failure {
  return { problem: isProblem(error) ? error : null, status: response?.status ?? 0 };
}

/** The one item a call gave back, or its failure. */
function one(
  data: { media: MediaItem[] } | undefined,
  error: unknown,
  response: Response,
): Reply<MediaItem> {
  const item = data?.media[0];
  return item === undefined ? failed(error, response) : { data: item };
}

/** Lists the library, newest upload first. */
export async function listMedia(query: MediaQuery): Promise<Reply<MediaPage>> {
  try {
    const { data, error, response } = await api.GET("/api/admin/v1/media/", {
      params: {
        query: {
          filter: query.missingAlt === true ? "kind:image+alt:null" : undefined,
          limit: MEDIA_PAGE.toString(),
          page: query.page?.toString(),
        },
      },
    });
    if (data === undefined) {
      return failed(error, response);
    }
    return { data: { media: data.media, pagination: data.meta.pagination } };
  } catch {
    return failed(null, undefined);
  }
}

/** Reads one item by its identity. */
export async function readMedia(id: string): Promise<Reply<MediaItem>> {
  try {
    const { data, error, response } = await api.GET("/api/admin/v1/media/{id}/", {
      params: { path: { id } },
    });
    return one(data, error, response);
  } catch {
    return failed(null, undefined);
  }
}

/** Saves a change to one item's details. */
export async function editMedia(id: string, edit: MediaEdit): Promise<Reply<MediaItem>> {
  try {
    const { data, error, response } = await api.PUT("/api/admin/v1/media/{id}/", {
      params: { path: { id } },
      body: { media: [edit] },
    });
    return one(data, error, response);
  } catch {
    return failed(null, undefined);
  }
}

/**
 * Sends `file` to the library, into `folder` or the top when empty. The
 * bytes go as they are, which the typed client cannot send, so this one
 * call is plain fetch.
 */
export async function uploadMedia(file: File, folder: string): Promise<Reply<Uploaded>> {
  const address = new URL("/api/admin/v1/media/", globalThis.location.origin);
  address.searchParams.set("name", file.name);
  if (folder !== "") {
    address.searchParams.set("folder", folder);
  }
  const headers = new Headers({ "Content-Type": "application/octet-stream" });
  const key = readKey();
  if (key !== null) {
    headers.set("Authorization", `Khalid ${key}`);
  }
  try {
    const body = new Uint8Array(await file.arrayBuffer());
    const response = await globalThis.fetch(
      new Request(address, { method: "POST", headers, body }),
    );
    const answer: unknown = await response.json().catch(() => null);
    const item = response.ok && hasItems(answer) ? answer.media[0] : undefined;
    if (item === undefined) {
      return failed(answer, response);
    }
    return { data: { item, duplicate: response.status === 200 } };
  } catch {
    return failed(null, undefined);
  }
}
