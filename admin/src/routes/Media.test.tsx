import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { item, listings, mediaBlog, mediaPage, pdf } from "../../test/media";
import { axeViolations, renderAt, signedIn } from "../../test/render";
import type { MediaItem } from "../api/media";
import { en } from "../strings/en";

const words = en.media;

/** A blog whose library holds `stored`. */
function library(stored: MediaItem[]) {
  return mediaBlog({ "GET /media/": () => mediaPage(stored) });
}

describe("the Media screen", () => {
  it("lists each file with its folder, kind and alt text", async () => {
    signedIn();
    library([item(), item({ id: "1b", alt: "A bar chart", folder: "guides/2026" }), pdf()]);
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.media }));
    const heading = await screen.findByRole("heading", { level: 1, name: words.title });
    expect(heading).toBeVisible();
    const table = await screen.findByRole("table", { name: words.library.title });
    const [, missing, described, file] = within(table).getAllByRole("row");
    expect(missing).toHaveTextContent("page-weight.png");
    expect(missing).toHaveTextContent("Image, 1600 × 900, 47 KB");
    expect(missing).toHaveTextContent(words.library.missing);
    expect(described).toHaveTextContent("guides/2026");
    expect(described).toHaveTextContent("A bar chart");
    expect(file).toHaveTextContent(words.library.top);
    expect(file).toHaveTextContent("File, 92 KB");
    expect(file).toHaveTextContent(words.library.notNeeded);
    expect(screen.getByText(words.library.found(3))).toBeVisible();
    expect(screen.getByRole("link", { name: en.nav.media })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(await axeViolations()).toEqual([]);
  });

  it("shows only images with no alt text at a press, and says when there are none", async () => {
    signedIn();
    const sent = library([]);
    renderAt("/media");
    expect(await screen.findByText(words.library.empty)).toBeVisible();
    const noAlt = screen.getByRole("button", { name: words.library.noAlt });
    await userEvent.click(noAlt);
    expect(await screen.findByText(words.library.allHaveAlt)).toBeVisible();
    expect(noAlt).toHaveAttribute("aria-pressed", "true");
    expect(listings(sent).at(-1)?.get("filter")).toBe("kind:image+alt:null");
  });

  it("pages through a long library", async () => {
    signedIn();
    const sent = mediaBlog({
      "GET /media/": (request) => {
        const page = Number(new URL(request.url).searchParams.get("page") ?? "1");
        return mediaPage([item()], 31, page);
      },
    });
    renderAt("/media");
    await userEvent.click(await screen.findByRole("link", { name: en.paging.next }));
    expect(await screen.findByText(en.paging.where(2, 2))).toBeVisible();
    expect(listings(sent).at(-1)?.get("page")).toBe("2");
  });
});
