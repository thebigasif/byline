import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { axeViolations, json, serve } from "../../../test/render";
import type { StockPicture, StockSource } from "../../api/pictures";
import { PictureSearch } from "./PictureSearch";
import { figureFor } from "./pictureBlock";

/** A photo as the blog's search answers it. */
function photo(changes: Partial<StockPicture> = {}): StockPicture {
  return {
    source: "unsplash",
    id: "Ab-12_c",
    description: "A boat on a still lake",
    preview: "https://images.unsplash.com/photo-1?w=400",
    address: "https://images.unsplash.com/photo-1?w=1080",
    width: 1080,
    height: 720,
    credit: "Ana Silva",
    credit_link: "https://unsplash.com/@ana?utm_source=khalid&utm_medium=referral",
    page: "https://unsplash.com/photos/Ab-12_c?utm_source=khalid&utm_medium=referral",
    ...changes,
  };
}

/** The search box for `source`, and what the writer picked. */
function searching(source: StockSource = "unsplash") {
  const onPick = vi.fn();
  const onCancel = vi.fn();
  const queries = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={queries}>
      <PictureSearch source={source} onPick={onPick} onCancel={onCancel} />
    </QueryClientProvider>,
  );
  const look = (words: string) =>
    fireEvent.change(screen.getByLabelText("Words to look for"), { target: { value: words } });
  return { onPick, onCancel, look };
}

describe("the figure a found picture becomes", () => {
  it("credits the photographer and Unsplash with links", () => {
    expect(figureFor(photo())).toEqual({
      src: "https://images.unsplash.com/photo-1?w=1080",
      alt: "A boat on a still lake",
      body: "Photo by [Ana Silva](https://unsplash.com/@ana?utm_source=khalid&utm_medium=referral) on [Unsplash](https://unsplash.com/photos/Ab-12_c?utm_source=khalid&utm_medium=referral)",
    });
  });

  it("says where a GIF came from", () => {
    const gif = photo({ source: "gifs", credit: "GIPHY", page: "https://giphy.com/gifs/x" });
    expect(figureFor(gif).body).toBe("Via [GIPHY](https://giphy.com/gifs/x)");
  });

  it("keeps a name with brackets from breaking the link, and always has alt text", () => {
    const odd = figureFor(photo({ credit: "Ana [AS] Silva", description: " " }));
    expect(odd.body).toContain("[Ana \\[AS\\] Silva]");
    expect(odd.alt).toBe("Photo by Ana [AS] Silva");
  });
});

describe("finding a picture", () => {
  it("says where photos load from before anything is picked", async () => {
    serve(() => json(200, { unsplash: true, gifs: false, pictures: [] }));
    searching();
    expect(screen.getByRole("form", { name: "Photo from Unsplash" })).toBeInTheDocument();
    expect(screen.getByText(/Photos stay on Unsplash and load from there/)).toBeVisible();
    expect(screen.getByLabelText("Words to look for")).toHaveFocus();
    expect(await axeViolations()).toEqual([]);
  });

  it("shows what the blog found and hands back the one clicked", async () => {
    const asked: string[] = [];
    serve((request) => {
      asked.push(request.url);
      return json(200, { unsplash: true, gifs: false, pictures: [photo()] });
    });
    const { onPick, look } = searching();
    look("mountain lake");
    const found = await screen.findByRole("button", { name: /A boat on a still lake, by Ana/ });
    expect(asked.some((url) => url.includes("source=unsplash"))).toBe(true);
    expect(asked.some((url) => url.includes("words=mountain"))).toBe(true);
    fireEvent.click(found);
    expect(onPick).toHaveBeenCalledWith(photo());
  });

  it("says so when nothing matches", async () => {
    serve(() => json(200, { unsplash: false, gifs: true, pictures: [] }));
    const { look } = searching("gifs");
    look("zzz");
    expect(await screen.findByText(/Nothing was found for those words/)).toBeInTheDocument();
  });

  it("passes on the blog's words when the search fails", async () => {
    serve(() =>
      json(503, {
        detail: "GIF search on GIPHY is turned off on this blog.",
        fix: "Ask the owner to turn it on under [media] in site.toml.",
      }),
    );
    const { look } = searching("gifs");
    look("dance");
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("turned off on this blog. Ask the owner to turn it on");
  });

  it("closes with Cancel", () => {
    serve(() => json(200, {}));
    const { onCancel } = searching();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalled();
  });
});
