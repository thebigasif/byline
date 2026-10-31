// The signed-in person's own passkeys. These routes read the session
// cookie the browser sends on its own: a passkey belongs to a person, so
// a tab signed in with a key alone gets a 401.
import { createApi } from "./client";
import { isProblem } from "./posts";
import type { Failure, Reply } from "./redirects";
import type { components } from "./schema";

/** One of the signed-in person's passkeys: its name and two dates, never its key. */
export type Passkey = components["schemas"]["PasskeyEntry"];

const api = createApi();

/** The reply to a call that failed: the problem the API sent, if any. */
function failed(error: unknown, response: Response): Failure {
  return { problem: isProblem(error) ? error : null, status: response.status };
}

/** The reply when the blog cannot be reached at all. */
const UNREACHABLE: Failure = { problem: null, status: 0 };

/** Lists the signed-in person's passkeys, oldest first. */
export async function listPasskeys(): Promise<Reply<Passkey[]>> {
  try {
    const { data, error, response } = await api.GET("/api/admin/v1/session/passkeys/");
    return data === undefined ? failed(error, response) : { data };
  } catch {
    return UNREACHABLE;
  }
}

/** Gives the passkey numbered `id` a new name. */
export async function renamePasskey(id: number, name: string): Promise<Reply<null>> {
  try {
    const { error, response } = await api.PATCH("/api/admin/v1/session/passkeys/{id}/", {
      params: { path: { id } },
      body: { name },
    });
    return response.ok ? { data: null } : failed(error, response);
  } catch {
    return UNREACHABLE;
  }
}

/** Removes the passkey numbered `id`, so it signs nobody in again. */
export async function removePasskey(id: number): Promise<Reply<null>> {
  try {
    const { error, response } = await api.DELETE("/api/admin/v1/session/passkeys/{id}/", {
      params: { path: { id } },
    });
    return response.ok ? { data: null } : failed(error, response);
  } catch {
    return UNREACHABLE;
  }
}
