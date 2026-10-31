// The key routes of the Admin API. They are typed by hand, field for field
// as `AdminKey` and `NewKey` in the G24 stack's OpenAPI document, because
// this document gains them only when that stack lands
// (docs/decisions/2026-10-31-admin-screens.md). Then these types become
// `components["schemas"]`.
import { call, nothing, type Reply, sending } from "./call";

/** A key that works. Its secret is in `key` only in the answer that made it. */
export type AdminKey = {
  /** The part before the secret, such as `1111222233334444`. */
  id: string;
  /** Who or what uses it, such as `SerpDad`. */
  name: string;
  /** `content`, `integration` or `staff`. */
  kind: string;
  /** A staff token's role, such as `editor`. */
  role?: string | null;
  /** An integration key's scopes, such as `posts:write`. */
  scopes: string[];
  created_at: string;
  /** The key itself, shown once. */
  key?: string | null;
};

/** The kinds of key the API makes; staff tokens come from signing in. */
export type NewKeyKind = "content" | "integration";

/** A new key: a content key takes no scopes. */
export type NewKey = { name: string; kind: NewKeyKind; scopes?: string[] };

async function keys(response: Response): Promise<AdminKey[]> {
  const body: { keys: AdminKey[] } = await response.json();
  return body.keys;
}

/** Every key that works, oldest first, never with its secret. */
export function listKeys(): Promise<Reply<AdminKey[]>> {
  return call("/keys/", {}, keys);
}

/** Makes a key; the answer holds the key itself this once. */
export function createKey(key: NewKey): Promise<Reply<AdminKey[]>> {
  return call("/keys/", sending("POST", { keys: [key] }), keys);
}

/** Switches a key off at once; the API refuses the key making the request. */
export function revokeKey(id: string): Promise<Reply<null>> {
  return call(`/keys/${encodeURIComponent(id)}/`, { method: "DELETE" }, nothing);
}
