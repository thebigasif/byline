import createClient, { type Client } from "openapi-fetch";
import { readKey } from "../auth/session";
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
    ...(fetch === undefined ? {} : { fetch }),
  });
  api.use({
    onRequest({ request }) {
      const current = key();
      if (current !== null) {
        request.headers.set("Authorization", `Khalid ${current}`);
      }
      return request;
    },
  });
  return api;
}
