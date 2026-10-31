import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { page, picture } from "../../../test/library";
import { axeViolations, json, serve } from "../../../test/render";
import { libraryFilter } from "../../api/library";
import { LibraryPicker } from "./LibraryPicker";

/** The picker, and what the writer picked. */
function picking() {
  const onPick = vi.fn();
  const onCancel = vi.fn();
  const queries = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={queries}>
      <LibraryPicker onPick={onPick} onCancel={onCancel} />
    </QueryClientProvider>,
  );
  return { onPick, onCancel };
}

describe("the library filter", () => {
  it("asks for images only", () => {
    expect(libraryFilter(" ")).toBe("kind:image");
  });

  it("looks for the words in the name or the alt text, quotes escaped", () => {
    expect(libraryFilter(" Asif's lake ")).toBe(
      "kind:image+(name:~'Asif\\'s lake',alt:~'Asif\\'s lake')",
    );
  });
});

describe("picking a picture from the library", () => {
  it("lists the library's images newest first and hands back the one clicked", async () => {
    const asked: URL[] = [];
    serve((request) => {
      asked.push(new URL(request.url));
      return json(200, page([picture()]));
    });
    const { onPick } = picking();
    expect(screen.getByLabelText("Words to look for")).toHaveFocus();
    const found = await screen.findByRole("button", { name: "A boat on a still lake" });
    expect(asked[0]?.pathname).toBe("/api/admin/v1/media/");
    expect(asked[0]?.searchParams.get("filter")).toBe("kind:image");
    expect(asked[0]?.searchParams.get("limit")).toBe("24");
    expect(await axeViolations()).toEqual([]);
    fireEvent.click(found);
    expect(onPick).toHaveBeenCalledWith(picture());
  });

  it("names a picture with no alt text by its file", async () => {
    serve(() => json(200, page([picture({ alt: "", name: "chart.png" })])));
    picking();
    expect(
      await screen.findByRole("button", { name: "No alt text yet: chart.png" }),
    ).toBeInTheDocument();
  });

  it("narrows the list to the words typed", async () => {
    const filters: (string | null)[] = [];
    serve((request) => {
      filters.push(new URL(request.url).searchParams.get("filter"));
      return json(200, page([]));
    });
    picking();
    fireEvent.change(screen.getByLabelText("Words to look for"), { target: { value: "boat" } });
    expect(await screen.findByText(/No picture's name or alt text has those words/)).toBeVisible();
    expect(filters).toContain("kind:image+(name:~'boat',alt:~'boat')");
  });

  it("shows the next page on request", async () => {
    const pages: (string | null)[] = [];
    serve((request) => {
      const asked = new URL(request.url).searchParams.get("page");
      pages.push(asked);
      return asked === "1"
        ? json(200, page([picture()], 2))
        : json(200, page([picture({ id: "fedcba9876543210", alt: "A red tram" })]));
    });
    picking();
    fireEvent.click(await screen.findByRole("button", { name: "Show more pictures" }));
    expect(await screen.findByRole("button", { name: "A red tram" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "A boat on a still lake" })).toBeInTheDocument();
    expect(pages).toEqual(["1", "2"]);
    expect(screen.queryByRole("button", { name: "Show more pictures" })).toBeNull();
  });

  it("says how to fill an empty library", async () => {
    serve(() => json(200, page([])));
    picking();
    expect(await screen.findByText(/Your media library has no pictures yet/)).toBeVisible();
  });

  it("says so when the blog does not answer", async () => {
    serve(() => json(500, {}));
    picking();
    expect(await screen.findByRole("alert")).toHaveTextContent(/could not be listed/);
  });

  it("closes with Cancel", () => {
    serve(() => json(200, page([])));
    const { onCancel } = picking();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalled();
  });
});
