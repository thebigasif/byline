import type { SearchState, StockSource } from "../api/pictures";

const names: Record<StockSource, string> = {
  unsplash: "Photos from Unsplash",
  gifs: "GIFs from GIPHY",
};

const settings: Record<StockSource, string> = { unsplash: "unsplash", gifs: "gifs" };

const variables: Record<StockSource, string> = {
  unsplash: "KHALID_UNSPLASH_ACCESS_KEY",
  gifs: "KHALID_GIPHY_API_KEY",
};

const slashes: Record<StockSource, string> = { unsplash: "/unsplash", gifs: "/gif" };

const states: Record<SearchState, string> = {
  off: "off",
  ready: "ready",
  missing: "the key is missing",
};

// The words for photo and GIF search in Settings, beside en.ts, which is
// near its size limit.
export const searches = {
  title: "Photo and GIF search",
  lede:
    "Whether writers can find free photos and GIFs from the editor. You change this in site.toml and on the server, because the keys never go through the browser.",
  loading: "Checking the searches",
  failed:
    "The blog did not answer, so it cannot say whether the searches work. Check the connection, then reload the page.",
  line: (source: StockSource, state: SearchState) => `${names[source]}: ${states[state]}.`,
  detail: (source: StockSource, state: SearchState) => {
    switch (state) {
      case "ready":
        return `Writers find them with ${slashes[source]} in a post.`;
      case "missing":
        return `Writers who search see a message instead of pictures. Set ${variables[source]} on the server and start khalid serve again.`;
      default:
        return `Add ${settings[source]} = true under [media] in site.toml to turn it on.`;
    }
  },
};
