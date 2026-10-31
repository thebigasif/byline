import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { item, mediaBlog, mediaPage, pdf } from "../../test/media";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { MediaEdit, MediaItem } from "../api/media";
import { en } from "../strings/en";

const words = en.media.details;

/** A blog holding `stored`, which saves each edit and moves `updated_at` on. */
function holding(stored: MediaItem, answer?: () => Response) {
  let current = stored;
  const edits: MediaEdit[] = [];
  const route = `/media/${stored.id}/`;
  const sent = mediaBlog({
    "GET /media/": () => mediaPage([current]),
    [`GET ${route}`]: () => json(200, { media: [current] }),
    [`PUT ${route}`]: async (request) => {
      const body: { media: MediaEdit[] } = await request.json();
      edits.push(...body.media);
      if (answer !== undefined) {
        return answer();
      }
      const edit = body.media[0];
      current = {
        ...current,
        alt: edit?.alt ?? current.alt,
        caption: edit?.caption ?? current.caption,
        folder: edit?.folder ?? current.folder,
        focal_point: edit?.focal_point ?? current.focal_point,
        updated_at: `2026-10-31T10:0${edits.length}:00Z`,
      };
      return json(200, { media: [current] });
    },
  });
  return { sent, edits };
}

describe("a file's details", () => {
  it("opens from the library and saves alt text on the copy it was read from", async () => {
    signedIn();
    const { edits } = holding(item());
    renderAt("/media");
    const open = await screen.findByRole("link", {
      name: en.media.library.edit("page-weight.png"),
    });
    await userEvent.click(open);
    expect(await screen.findByRole("heading", { level: 1, name: "page-weight.png" })).toBeVisible();
    const lede = words.uploaded("Image, 1600 × 900, 47 KB", "asif", "5 October 2026");
    expect(screen.getByText(lede)).toBeVisible();
    await userEvent.type(screen.getByLabelText(words.alt), "A bar chart of page weight");
    await userEvent.click(screen.getByRole("button", { name: words.save }));
    expect(await screen.findByText(words.saved)).toBeVisible();
    expect(edits).toEqual([{ updated_at: item().updated_at, alt: "A bar chart of page weight" }]);
    expect(await axeViolations()).toEqual([]);
  });

  it("moves the focal point by a click on the picture or by its numbers", async () => {
    signedIn();
    const { edits } = holding(item());
    renderAt(`/media/${item().id}`);
    const picture = await screen.findByRole("button", { name: words.picture });
    // jsdom lays nothing out, so the picture is given a size of 200 by 100.
    picture.getBoundingClientRect = () => ({
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      right: 200,
      bottom: 100,
      width: 200,
      height: 100,
      toJSON: () => ({}),
    });
    fireEvent.click(picture, { clientX: 50, clientY: 75, detail: 1 });
    expect(screen.getByLabelText(words.left)).toHaveValue(25);
    expect(screen.getByLabelText(words.top)).toHaveValue(75);
    await userEvent.clear(screen.getByLabelText(words.left));
    await userEvent.type(screen.getByLabelText(words.left), "62");
    await userEvent.click(screen.getByRole("button", { name: words.save }));
    expect(await screen.findByText(words.saved)).toBeVisible();
    expect(edits[0]?.focal_point).toEqual({ x: 620, y: 750 });
  });

  it("makes a second save on the copy the first one gave back", async () => {
    signedIn();
    const { edits } = holding(item());
    renderAt(`/media/${item().id}`);
    await userEvent.type(await screen.findByLabelText(words.caption), "Page weight");
    await userEvent.click(screen.getByRole("button", { name: words.save }));
    expect(await screen.findByText(words.saved)).toBeVisible();
    await userEvent.type(screen.getByLabelText(words.caption), " in 2026");
    await userEvent.click(screen.getByRole("button", { name: words.save }));
    await waitFor(() => expect(edits).toHaveLength(2));
    expect(edits.map((edit) => edit.updated_at)).toEqual([
      item().updated_at,
      "2026-10-31T10:01:00Z",
    ]);
    expect(edits[1]).toEqual({
      updated_at: "2026-10-31T10:01:00Z",
      caption: "Page weight in 2026",
    });
  });

  it("asks a file for no alt text or focal point, and moves it to another folder", async () => {
    signedIn();
    const { edits } = holding(pdf());
    renderAt(`/media/${pdf().id}`);
    const folder = await screen.findByLabelText(en.media.library.folder);
    expect(screen.queryByLabelText(words.alt)).toBeNull();
    expect(screen.queryByRole("button", { name: words.picture })).toBeNull();
    await userEvent.type(folder, "guides");
    await userEvent.click(screen.getByRole("button", { name: words.save }));
    expect(await screen.findByText(words.saved)).toBeVisible();
    expect(edits).toEqual([{ updated_at: pdf().updated_at, folder: "guides" }]);
  });

  it("shows the API's words when someone else changed the file first", async () => {
    signedIn();
    const stale = {
      type: "https://khalid.dev/problems/conflict",
      title: "sam changed this item at 2026-10-30 09:00 UTC, after the copy you edited.",
      status: 409,
      detail: "Saving your copy would undo their change.",
      fix: "Fetch the item again, make your change to it, and send its new updated_at.",
    };
    holding(item(), () => json(409, stale));
    renderAt(`/media/${item().id}`);
    await userEvent.type(await screen.findByLabelText(words.alt), "A chart");
    await userEvent.click(screen.getByRole("button", { name: words.save }));
    expect(await screen.findByRole("alert")).toHaveTextContent(stale.title);
    expect(screen.queryByText(words.saved)).toBeNull();
  });

  it("says so when no file has this identity", async () => {
    signedIn();
    mediaBlog({ "GET /media/0000/": () => json(404, {}) });
    renderAt("/media/0000");
    const heading = await screen.findByRole("heading", { level: 1 });
    expect(heading).toHaveTextContent(en.problems.noMediaItem.what);
    expect(screen.getByRole("link", { name: words.back })).toBeVisible();
  });
});
