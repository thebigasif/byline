import createClient, { type Client } from "openapi-fetch";
import { readCsrf, readKey } from "../auth/session";
import type { paths } from "./schema";

/** A client for the Admin and Content APIs, typed from the OpenAPI document. */
export type Api = Client<paths>;

type Options = {
  /** Where the key comes from on each request; the signed-in tab by default. */
  key?: () => string | null;
  fetch?: typeof globalThis.fetch;
};

/** Makes a client that talks to the blog that served the admin. */
export function createApi({ key = readKey, fetch }: Options = {}): Api {
  const api = createClient<paths>({
    baseUrl: globalThis.location.origin,
    // Looked up on each request, so the page always uses the current fetch.
    fetch: fetch ?? ((request: Request) => globalThis.fetch(request)),
  });
  api.use({
    onRequest({ request }) {
      const current = key();
      const csrf = readCsrf();
      if (current !== null) {
        request.headers.set("Authorization", `Khalid ${current}`);
      } else if (csrf !== null) {
        request.headers.set("x-khalid-csrf", csrf);
      }
      return request;
    },
  });
  return api;
}
