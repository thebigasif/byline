import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { writing } from "../../../test/editor";
import { page, picture } from "../../../test/library";
import { json, serve } from "../../../test/render";
import { libraryFigure } from "./pictureBlock";

describe("the figure a library picture becomes", () => {
  it("keeps the alt text and caption saved in the library", () => {
    expect(libraryFigure(picture({ caption: " Lake Bled at dawn " }))).toEqual({
      src: "https://blog.example/media/library/0123456789abcdef/lake.jpg",
      alt: "A boat on a still lake",
      body: "Lake Bled at dawn",
    });
  });
});

describe("a library picture from the block menu", () => {
  it("is offered with both searches off and adds a figure with the library's words", async () => {
    serve((request) =>
      new URL(request.url).pathname.endsWith("/media/search/")
        ? json(200, { unsplash: false, gifs: false, pictures: [] })
        : json(200, page([picture()])),
    );
    const { type, press, onChange } = writing();
    type("/libr");
    await screen.findByRole("button", { name: /Picture from your library/ });
    press("Enter");
    fireEvent.click(await screen.findByRole("button", { name: "A boat on a still lake" }));
    const figure =
      ':::figure{src="https://blog.example/media/library/0123456789abcdef/lake.jpg" alt="A boat on a still lake"}\n' +
      "Lake Bled at dawn\n" +
      ":::";
    expect(onChange).toHaveBeenLastCalledWith(expect.stringContaining(figure));
    expect(screen.queryByLabelText("Words to look for")).toBeNull();
  });
});
