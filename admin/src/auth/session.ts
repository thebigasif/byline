// The signed-in person's key lives in this tab's session storage only:
// it is gone when the tab closes, and never sent anywhere but this blog.
const STORAGE_NAME = "khalid-admin-key";

/** The key this tab signed in with, if any. */
export function readKey(): string | null {
  return sessionStorage.getItem(STORAGE_NAME);
}

/** Keeps the key for this tab after the blog accepted it. */
export function saveKey(key: string): void {
  sessionStorage.setItem(STORAGE_NAME, key);
}

/** Forgets the key, which signs this tab out. */
export function forgetKey(): void {
  sessionStorage.removeItem(STORAGE_NAME);
}
