// One Admin API request with the signed-in key, for the screens whose
// answers need no more than "the data, or the problem".
import { signingHeaders } from "../auth/session";
import { isProblem, type Problem } from "./posts";

/** A call that failed: the problem the API sent, if any, and the status; 0 when unreachable. */
export type Failure = { problem: Problem | null; status: number };

/** What a call gave back: its data, or the problem. */
export type Reply<T> = { data: T } | Failure;

/** Whether a call gave back its data. */
export function isData<T>(reply: Reply<T> | undefined): reply is { data: T } {
  return reply !== undefined && "data" in reply;
}

const ADMIN = "/api/admin/v1";

/** Sends one request to the blog with the signed-in key, and reads what came back. */
export async function call<T>(
  path: string,
  init: RequestInit,
  read: (response: Response) => Promise<T>,
): Promise<Reply<T>> {
  const request = new Request(`${globalThis.location.origin}${ADMIN}${path}`, init);
  for (const [name, value] of Object.entries(signingHeaders())) {
    request.headers.set(name, value);
  }
  try {
    const response = await globalThis.fetch(request);
    if (response.ok) {
      return { data: await read(response) };
    }
    const body: unknown = await response.json().catch(() => null);
    return { problem: isProblem(body) ? body : null, status: response.status };
  } catch {
    return { problem: null, status: 0 };
  }
}

/** A request that sends `body` as JSON. */
export function sending(method: string, body: unknown): RequestInit {
  return { method, headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) };
}

/** Reads nothing, for answers such as 204. */
export async function nothing(): Promise<null> {
  return null;
}
