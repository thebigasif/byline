import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { axeViolations, json, serve } from "../../test/render";
import { stateOf } from "../api/pictures";
import { PictureSearches } from "./PictureSearches";

/** The section, on a blog whose search route answers `body` with `status`. */
function settingsWith(status: number, body: unknown) {
  const asked: string[] = [];
  serve((request) => {
    asked.push(`${request.method} ${new URL(request.url).pathname}`);
    return json(status, body);
  });
  const queries = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={queries}>
      <PictureSearches />
    </QueryClientProvider>,
  );
  return asked;
}

describe("a search's state", () => {
  it("is off, ready, or missing its key", () => {
    expect(stateOf(false, true)).toBe("off");
    expect(stateOf(true, true)).toBe("ready");
    expect(stateOf(true, false)).toBe("missing");
  });
});

describe("photo and GIF search in Settings", () => {
  it("says one is ready and the other misses its key, with what to set", async () => {
    const asked = settingsWith(200, {
      unsplash: true,
      gifs: true,
      keys: { unsplash: true, gifs: false },
      pictures: [],
    });
    expect(screen.getByRole("heading", { name: "Photo and GIF search" })).toBeVisible();
    expect(await screen.findByText("Photos from Unsplash: ready.")).toBeVisible();
    expect(screen.getByText(/with \/unsplash in a post/)).toBeVisible();
    const missing = screen.getByText("GIFs from GIPHY: the key is missing.");
    expect(missing.closest("li")).toHaveClass("careful");
    expect(screen.getByText(/Set KHALID_GIPHY_API_KEY on the server/)).toBeVisible();
    expect(asked).toEqual(["GET /api/admin/v1/media/search/"]);
    expect(await axeViolations()).toEqual([]);
  });

  it("says how to turn on a search that is off, and offers no field for a key", async () => {
    settingsWith(200, {
      unsplash: false,
      gifs: false,
      keys: { unsplash: false, gifs: false },
      pictures: [],
    });
    expect(await screen.findByText("Photos from Unsplash: off.")).toBeVisible();
    expect(screen.getByText(/Add gifs = true under \[media\] in site.toml/)).toBeVisible();
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("says so when the blog does not answer", async () => {
    settingsWith(500, {});
    expect(await screen.findByRole("alert")).toHaveTextContent(/cannot say whether the searches/);
  });
});
