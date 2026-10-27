import { createMemoryHistory, RouterProvider } from "@tanstack/react-router";
import { render } from "@testing-library/react";
import axe from "axe-core";
import { makeRouter } from "../src/routes/router";

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
