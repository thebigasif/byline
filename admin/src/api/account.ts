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

/** Who this tab's session cookie signs in; a 401 when the tab signed in with a token. */
export function readSession(): Promise<Reply<SessionAnswer>> {
  return call("/session/", {}, session);
}

/** Starts adding a passkey: what to pass to `navigator.credentials.create`. */
export function passkeyOptions(): Promise<Reply<components["schemas"]["CreateOptions"]>> {
  return call("/session/passkeys/options/", { method: "POST" }, (response) => response.json());
}

/** Adds the passkey the browser made to the signed-in account. */
export function addPasskey(passkey: components["schemas"]["NewPasskey"]): Promise<Reply<null>> {
  return call("/session/passkeys/", sending("POST", passkey), nothing);
}

/** Changes the password, and ends every other session; `current` is null for a first one. */
export function changePassword(current: string | null, next: string): Promise<Reply<null>> {
  const body: components["schemas"]["PasswordChange"] = { current, new: next };
  return call("/session/password/", sending("PUT", body), nothing);
}

/** Ends every session of the account, this one too. */
export function signOutEverywhere(): Promise<Reply<null>> {
  return call("/sessions/", { method: "DELETE" }, nothing);
}

/** Ends this tab's email sign-in on the server too, and clears its cookie. */
export function endSession(): Promise<Reply<null>> {
  return call("/session/", { method: "DELETE" }, nothing);
}
