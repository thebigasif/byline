import { call, nothing, type Reply, sending } from "./call";
import type { components } from "./schema";

/** A signed-in session: whose it is, and the CSRF token its requests carry. */
export type SessionAnswer = components["schemas"]["SessionAnswer"];

async function session(response: Response): Promise<SessionAnswer> {
  return response.json();
}

/** Signs in with an email and password; the browser keeps the session cookie. */
export function signInWithEmail(email: string, password: string): Promise<Reply<SessionAnswer>> {
  const body: components["schemas"]["SignInBody"] = { email, password };
  return call("/session/", sending("POST", body), session);
}

/** Signs in with the token from a sign-in link; the browser keeps the session cookie. */
export function redeemLink(token: string): Promise<Reply<SessionAnswer>> {
  const body: components["schemas"]["LinkRedeem"] = { token };
  return call("/session/link/redeem/", sending("POST", body), session);
}

/** Ends this tab's email sign-in on the server too, and clears its cookie. */
export function endSession(): Promise<Reply<null>> {
  return call("/session/", { method: "DELETE" }, nothing);
}
