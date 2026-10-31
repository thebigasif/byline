import { describe, expect, it } from "vitest";
import { item, mediaPage } from "../../test/media";
import { json, serve, signedIn } from "../../test/render";
import { editMedia, listMedia, readMedia, uploadMedia } from "./media";

/** A blog that answers every call with `answer`, and the requests it was sent. */
function blog(answer: () => Response) {
  const sent: Request[] = [];
  serve((request) => {
    sent.push(request);
    return answer();
  });
  return sent;
}

const STALE = {
  type: "https://khalid.dev/problems/conflict",
  title: "sam changed this item at 2026-10-30 09:00 UTC, after the copy you edited.",
  status: 409,
  detail: "Saving your copy would undo their change.",
  fix: "Fetch the item again, make your change to it, and send its new updated_at.",
};

describe("the media client", () => {
  it("lists a page of the library, and only images with no alt text when asked", async () => {
    const sent = blog(() => mediaPage([item()], 31, 2));
    const reply = await listMedia({ missingAlt: true, page: 2 });
    const url = new URL(sent[0]?.url ?? "http://localhost/");
    expect(url.pathname).toBe("/api/admin/v1/media/");
    // A bare + would reach the server as a space and change the filter.
    expect(url.search).toContain("%2B");
    expect(url.searchParams.get("filter")).toBe("kind:image+alt:null");
    expect(url.searchParams.get("limit")).toBe("30");
    expect(url.searchParams.get("page")).toBe("2");
    expect(reply).toEqual({
      data: {
        media: [item()],
        pagination: { page: 2, limit: 30, pages: 2, total: 31, next: null, prev: 1 },
      },
    });
  });

  it("lists every file when no filter is asked for", async () => {
    const sent = blog(() => mediaPage([]));
    await listMedia({});
    const url = new URL(sent[0]?.url ?? "http://localhost/");
    expect(url.searchParams.has("filter")).toBe(false);
    expect(url.searchParams.has("page")).toBe(false);
  });

  it("reads one item and saves an edit made on its copy", async () => {
    const sent = blog(() => json(200, { media: [item({ alt: "A bar chart" })] }));
    expect(await readMedia("3f2a9c1e0b57d0aa")).toEqual({ data: item({ alt: "A bar chart" }) });
    const edit = { updated_at: item().updated_at, alt: "A bar chart" };
    expect(await editMedia("3f2a9c1e0b57d0aa", edit)).toEqual({
      data: item({ alt: "A bar chart" }),
    });
    expect(sent[1]?.method).toBe("PUT");
    expect(new URL(sent[1]?.url ?? "").pathname).toBe("/api/admin/v1/media/3f2a9c1e0b57d0aa/");
    expect(await sent[1]?.json()).toEqual({ media: [edit] });
  });

  it("hands back the problem an edit on an old copy gets", async () => {
    blog(() => json(409, STALE));
    const edit = { updated_at: item().updated_at, caption: "Page weight" };
    expect(await editMedia("3f2a9c1e0b57d0aa", edit)).toEqual({ problem: STALE, status: 409 });
  });

  it("uploads a file's bytes into a folder with the signed-in key", async () => {
    signedIn();
    const sent = blog(() => json(201, { media: [item()] }));
    const file = new File(["not really a png"], "Page Weight.png", { type: "image/png" });
    expect(await uploadMedia(file, "guides")).toEqual({
      data: { item: item(), duplicate: false },
    });
    const request = sent[0];
    const url = new URL(request?.url ?? "");
    expect(request?.method).toBe("POST");
    expect(url.searchParams.get("name")).toBe("Page Weight.png");
    expect(url.searchParams.get("folder")).toBe("guides");
    expect(request?.headers.get("Content-Type")).toBe("application/octet-stream");
    expect(request?.headers.get("Authorization")).toBe("Khalid ks_0123456789abcdef_test");
    expect(await request?.text()).toBe("not really a png");
  });

  it("says when the library already had the file", async () => {
    const sent = blog(() => json(200, { media: [item()] }));
    const file = new File(["png"], "copy.png");
    expect(await uploadMedia(file, "")).toEqual({ data: { item: item(), duplicate: true } });
    expect(new URL(sent[0]?.url ?? "").searchParams.has("folder")).toBe(false);
  });

  it("hands back why a file was refused", async () => {
    const refused = {
      type: "https://khalid.dev/problems/invalid-body",
      title: "The file was not added: its content is not a type the library keeps.",
      status: 422,
      detail: "The library keeps only types it can check by their content.",
      fix: "Send a JPEG, PNG or WebP image.",
    };
    blog(() => json(422, refused));
    const file = new File(["MZ"], "photo.jpg");
    expect(await uploadMedia(file, "")).toEqual({ problem: refused, status: 422 });
  });

  it("answers status 0 with no problem when the blog cannot be reached", async () => {
    serve(() => {
      throw new TypeError("Failed to fetch");
    });
    expect(await listMedia({})).toEqual({ problem: null, status: 0 });
    expect(await uploadMedia(new File(["png"], "a.png"), "")).toEqual({
      problem: null,
      status: 0,
    });
  });
});
