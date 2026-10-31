// The signed-in person's key lives in this tab's session storage only:
// it is gone when the tab closes, and never sent anywhere but this blog.
// Someone who signed in with their email has no key: the browser keeps
// their session cookie, and the tab keeps the CSRF token every request
// with that cookie must carry (RULES.md BL-SEC-9).
const STORAGE_NAME = "khalid-admin-key";
const CSRF_NAME = "khalid-admin-csrf";

/** The key this tab signed in with, if any. */
export function readKey(): string | null {
  return sessionStorage.getItem(STORAGE_NAME);
}

/** Keeps the key for this tab after the blog accepted it. */
export function saveKey(key: string): void {
  sessionStorage.setItem(STORAGE_NAME, key);
}

/** The CSRF token of the email sign-in this tab made, if any. */
export function readCsrf(): string | null {
  return sessionStorage.getItem(CSRF_NAME);
}

/** Keeps the CSRF token of an email sign-in for this tab. */
export function saveCsrf(csrf: string): void {
  sessionStorage.setItem(CSRF_NAME, csrf);
}

/** Forgets the key and the CSRF token, which signs this tab out. */
export function forgetKey(): void {
  sessionStorage.removeItem(STORAGE_NAME);
  sessionStorage.removeItem(CSRF_NAME);
}
