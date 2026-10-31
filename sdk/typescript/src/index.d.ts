import type { Operations, Problem } from "./schema.js";

export type * from "./schema.js";

/** The name of one API operation, its `operationId` in `openapi.json`. */
export type OperationId = keyof Operations;

/** What one call sends: path, query and header values, and the body. */
export type Input<K extends OperationId> = Operations[K]["input"];

/** What one call answers on success. */
export type Output<K extends OperationId> = Operations[K]["output"];

/** Where one operation is sent and how. */
export interface Route {
  /** The HTTP method, such as `GET`. */
  readonly method: string;
  /** The path template, such as `/api/content/v1/posts/slug/{slug}/`. */
  readonly path: string;
  /** The key that signs the call: the Admin API key, the content key, or none. */
  readonly key: "admin" | "content" | "none";
  /** The body's media type, or `null` when the operation takes no body. */
  readonly body: string | null;
}

/** Every operation's route. */
export type Routes = { readonly [K in OperationId]: Route };

/** How the client reaches a Khalid site and signs its calls. */
export interface ClientOptions {
  /** The site's address, such as `https://example.com`. The API lives under `/api/` on it. */
  baseUrl: string;
  /** An integration key or staff token for the Admin API. Sent as `Authorization: Khalid <key>`. */
  adminKey?: string;
  /** The site's content key for the Content API. Sent as `Authorization: Khalid-Content <key>`. */
  contentKey?: string;
  /** The `fetch` to send with. The global one when left out. */
  fetch?: typeof fetch;
}

/**
 * What a call answered: the data on success, or the problem the server
 * described (RFC 9457) with what happened, why and what to do.
 */
export type Result<T> =
  | { readonly ok: true; readonly status: number; readonly data: T }
  | { readonly ok: false; readonly status: number; readonly problem: Problem };

/** A client for one Khalid site. */
export interface Client {
  /**
   * Sends one operation and reads its answer.
   *
   * Pass the values the operation takes, or `{}` when it takes none.
   */
  request<K extends OperationId>(operation: K, input: Input<K>): Promise<Result<Output<K>>>;
}

/** Makes a client for the site at `options.baseUrl`. */
export function createClient(options: ClientOptions): Client;
