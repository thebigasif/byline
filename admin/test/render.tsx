import { createMemoryHistory, RouterProvider } from "@tanstack/react-router";
import { render } from "@testing-library/react";
import axe from "axe-core";
import { vi } from "vitest";
import { saveKey } from "../src/auth/session";
import { makeRouter } from "../src/routes/router";

/** Signs the tab in with a made-up token, as if the blog had accepted it. */
export function signedIn() {
  saveKey("ks_0123456789abcdef_test");
}

/** Makes the blog's server answer every request with this status. */
export function serverAnswers(status: number) {
  const fetch = vi.fn(
    async () => new Response("{}", { status, headers: { "Content-Type": "application/json" } }),
  );
  vi.stubGlobal("fetch", fetch);
  return fetch;
}

/** Shows the admin at one address under /admin, as a browser tab would. */
export function renderAt(path: string) {
  const router = makeRouter(createMemoryHistory({ initialEntries: [`/admin${path}`] }));
  return { router, ...render(<RouterProvider router={router} />) };
}

/** The accessibility problems axe finds on the page; colour needs a real browser. */
export async function axeViolations(): Promise<string[]> {
  const result = await axe.run(document.body, {
    rules: { "color-contrast": { enabled: false } },
  });
  return result.violations.map((violation) => violation.id);
}
