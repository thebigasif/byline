import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { writing } from "../../../test/editor";
import { json, serve } from "../../../test/render";

const lake = {
  source: "unsplash",
  id: "Ab-12_c",
  description: "A boat on a still lake",
  preview: "https://images.unsplash.com/photo-1?w=400",
  address: "https://images.unsplash.com/photo-1?w=1080",
  width: 1080,
  height: 720,
  credit: "Ana Silva",
  credit_link: "https://unsplash.com/@ana",
  page: "https://unsplash.com/photos/Ab-12_c",
};

/** A blog with photo search on as `unsplash` says, and the requests it saw. */
function blogWith(unsplash: boolean) {
  const seen: string[] = [];
  serve(async (request) => {
    seen.push(`${request.method} ${new URL(request.url).pathname}`);
    if (request.method === "POST") {
      return json(200, await request.json());
    }
    return json(200, { unsplash, gifs: false, pictures: [lake] });
  });
  return seen;
}

describe("a photo from the block menu", () => {
  it("finds a photo and adds it with its credit", async () => {
    const seen = blogWith(true);
    const { type, press, onChange } = writing();
    type("/unspl");
    await screen.findByRole("button", { name: /Photo from Unsplash/ });
    press("Enter");
    const box = await screen.findByLabelText("Words to look for");
    fireEvent.change(box, { target: { value: "lake" } });
    fireEvent.click(await screen.findByRole("button", { name: /A boat on a still lake, by Ana/ }));
    const figure =
      ':::figure{src="https://images.unsplash.com/photo-1?w=1080" alt="A boat on a still lake"}\n' +
      "Photo by [Ana Silva](https://unsplash.com/@ana) on [Unsplash](https://unsplash.com/photos/Ab-12_c)\n" +
      ":::";
    expect(onChange).toHaveBeenLastCalledWith(expect.stringContaining(figure));
    expect(screen.queryByLabelText("Words to look for")).toBeNull();
    await screen.findByRole("textbox");
    await vi.waitFor(() => expect(seen).toContain("POST /api/admin/v1/media/search/used/"));
  });

  it("is not offered while the owner has not turned it on", async () => {
    const seen = blogWith(false);
    const { type } = writing();
    type("/unspl");
    await screen.findByText(/No block has that name/);
    await vi.waitFor(() => expect(seen).toContain("GET /api/admin/v1/media/search/"));
    expect(screen.queryByRole("button", { name: /Photo from Unsplash/ })).toBeNull();
  });
});
