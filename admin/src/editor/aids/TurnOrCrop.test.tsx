import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { axeViolations, json, serve } from "../../../test/render";
import { BLOCK_SPECS } from "../blocks/specs";
import { BlockForm } from "./BlockForm";
import { cropFor, libraryKey, turned } from "./pictureTurn";

const KEY = "3f2a9c1e0b57d0aa";
const PHOTO = `https://www.example.com/blog/media/library/${KEY}/team-photo.png`;
const COPY = "https://www.example.com/blog/media/library/0b57d0aa3f2a9c1e/team-photo.png";

describe("turning and cropping, worked out", () => {
  it("finds the library key in a picture's address, and only there", () => {
    expect(libraryKey(PHOTO)).toBe(KEY);
    expect(libraryKey(`/blog/media/library/${KEY}/team-photo.png`)).toBe(KEY);
    expect(libraryKey("https://images.unsplash.com/photo-1?w=1080")).toBeNull();
    expect(libraryKey("/blog/media/library/not-a-key/team-photo.png")).toBeNull();
  });

  it("turns in quarter turns both ways", () => {
    expect(turned(0, true)).toBe(90);
    expect(turned(0, false)).toBe(270);
    expect(turned(270, true)).toBe(0);
  });

  it("crops to a shape around the focal point, as far as the edges allow", () => {
    const wide = { width: 400, height: 200, x: 500, y: 500 };
    expect(cropFor("taken", 0, wide)).toBeNull();
    expect(cropFor("square", 0, wide)).toEqual({ left: 250, top: 0, width: 500, height: 1000 });
    const leftEdge = { ...wide, x: 0 };
    expect(cropFor("square", 0, leftEdge)).toEqual({ left: 0, top: 0, width: 500, height: 1000 });
    expect(cropFor("square", 90, wide)).toEqual({ left: 0, top: 250, width: 1000, height: 500 });
  });
});

/** The figure's details form for a picture at `src`, and what it saved. */
function figureForm(src: string) {
  const figure = BLOCK_SPECS.find((spec) => spec.name === "figure");
  if (figure === undefined) {
    throw new Error("the figure block is in the specs");
  }
  const values = { src, alt: "The team at the summer meetup", body: "" };
  const onSave = vi.fn();
  const queries = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={queries}>
      <BlockForm
        details={{ spec: figure, values, at: 0, editing: true }}
        onSave={onSave}
        onCancel={vi.fn()}
      />
    </QueryClientProvider>,
  );
  return { values, onSave };
}

/** A blog that knows the photo and makes copies, and the edits it was sent. */
function blogWithPhoto() {
  const edits: unknown[] = [];
  serve(async (request) => {
    if (request.method === "POST") {
      edits.push(await request.json());
      return json(201, { media: [{ id: "0b57d0aa3f2a9c1e", url: COPY }] });
    }
    const item = { id: KEY, url: PHOTO, width: 400, height: 200, focal_point: { x: 500, y: 500 } };
    return json(200, { media: [item] });
  });
  return edits;
}

describe("turning or cropping a picture in a post", () => {
  it("makes a turned, square copy and puts it in the post", async () => {
    const edits = blogWithPhoto();
    const { values, onSave } = figureForm(PHOTO);
    fireEvent.click(screen.getByRole("button", { name: "Turn or crop" }));
    expect(screen.getByRole("heading", { name: "Turn or crop the picture" })).toBeVisible();
    expect(screen.getByText(/The original stays/)).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Turn right" }));
    fireEvent.click(await screen.findByRole("radio", { name: "Square" }));
    expect(await axeViolations()).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: "Save as a new copy" }));
    await vi.waitFor(() => expect(onSave).toHaveBeenCalledWith({ ...values, src: COPY }));
    expect(edits).toEqual([{ turn: 90, crop: { left: 0, top: 250, width: 1000, height: 500 } }]);
  });

  it("goes back to the details with Cancel", () => {
    blogWithPhoto();
    figureForm(PHOTO);
    fireEvent.click(screen.getByRole("button", { name: "Turn or crop" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.getByRole("button", { name: "Turn or crop" })).toBeVisible();
  });

  it("offers nothing for a picture the library does not keep", () => {
    serve(() => json(200, {}));
    figureForm("https://images.unsplash.com/photo-1?w=1080");
    expect(screen.queryByRole("button", { name: "Turn or crop" })).toBeNull();
  });
});
