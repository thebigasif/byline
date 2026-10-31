import { createApi } from "./client";
import { isProblem } from "./posts";
import type { Reply } from "./redirects";
import type { components } from "./schema";

/** Whether SerpDad is connected, and the drafts it sent, newest first. */
export type SerpdadState = components["schemas"]["SerpdadState"];
/** A draft SerpDad sent, with its checks on arrival and where it opens. */
export type SerpdadDraft = components["schemas"]["SerpdadDraft"];

const api = createApi();

const ROUTE = "/api/admin/v1/serpdad/";

/** The data, or the problem the API sent; status 0 when the blog was unreachable. */
function reply<T>(data: T | undefined, error: unknown, response: Response): Reply<T> {
  if (response.ok && data !== undefined) {
    return { data };
  }
  return { problem: isProblem(error) ? error : null, status: response.status };
}

const UNREACHABLE = { problem: null, status: 0 };

/** Reads whether SerpDad is connected and the last 20 drafts it sent. */
export async function readSerpdad(): Promise<Reply<SerpdadState>> {
  try {
    const { data, error, response } = await api.GET(ROUTE);
    return reply(data, error, response);
  } catch {
    return UNREACHABLE;
  }
}

/** Makes SerpDad's key, replacing any old one; the answer holds the key this once. */
export async function connectSerpdad(): Promise<Reply<SerpdadState>> {
  try {
    const { data, error, response } = await api.POST(ROUTE);
    return reply(data, error, response);
  } catch {
    return UNREACHABLE;
  }
}

/** Switches SerpDad's key off at once; the drafts it sent stay. */
export async function disconnectSerpdad(): Promise<Reply<null>> {
  try {
    const { error, response } = await api.DELETE(ROUTE);
    return reply(response.ok ? null : undefined, error, response);
  } catch {
    return UNREACHABLE;
  }
}
