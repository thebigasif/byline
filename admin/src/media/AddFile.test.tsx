import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { item, mediaBlog, mediaPage, pdf } from "../../test/media";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { MediaItem } from "../api/media";
import { en } from "../strings/en";

const words = en.media;

/** A blog whose library holds `stored`, and which adds what is uploaded with `status`. */
function library(stored: MediaItem[], status = 201) {
  const media = [...stored];
  return mediaBlog({
    "GET /media/": () => mediaPage(media),
    "POST /media/": () => {
      const added = item({ id: "77aa", name: "topic-cluster-map.png" });
      if (status === 201) {
        media.unshift(added);
      }
      return json(status, { media: [added] });
    },
  });
}

/** Picks a file and a folder in the upload form and presses Upload. */
async function upload(folder = "") {
  const file = new File(["png"], "Topic Cluster Map.png", { type: "image/png" });
  await userEvent.upload(await screen.findByLabelText(words.add.file), file);
  if (folder !== "") {
    await userEvent.type(screen.getByLabelText(words.add.folder), folder);
  }
  await userEvent.click(screen.getByRole("button", { name: words.add.submit }));
}

describe("adding a file", () => {
  it("uploads a file into a folder and lists it, asking for its alt text", async () => {
    signedIn();
    const sent = library([pdf()]);
    renderAt("/media");
    await upload("guides");
    const added = `${words.add.added("topic-cluster-map.png")} ${words.add.addAlt}`;
    expect(await screen.findByText(added)).toBeVisible();
    const posted = sent.find((request) => request.method === "POST");
    expect(new URL(posted?.url ?? "").searchParams.get("folder")).toBe("guides");
    const table = await screen.findByRole("table", { name: words.library.title });
    await waitFor(() => expect(table).toHaveTextContent("topic-cluster-map.png"));
    expect(await axeViolations()).toEqual([]);
  });

  it("says when the library already had the file", async () => {
    signedIn();
    library([item()], 200);
    renderAt("/media");
    await upload();
    const duplicate = words.add.duplicate("topic-cluster-map.png");
    expect(await screen.findByText(duplicate)).toBeVisible();
  });

  it("asks for a file before uploading anything", async () => {
    signedIn();
    const sent = library([]);
    renderAt("/media");
    await userEvent.click(await screen.findByRole("button", { name: words.add.submit }));
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.noMedia.what);
    expect(sent.filter((request) => request.method === "POST")).toEqual([]);
  });

  it("shows the API's words when a file is refused", async () => {
    signedIn();
    const refused = {
      type: "https://khalid.dev/problems/invalid-body",
      title: "The file was not added: its content is not a type the library keeps.",
      status: 422,
      detail: "The library keeps only types it can check by their content.",
      fix: "Send a JPEG, PNG or WebP image.",
    };
    mediaBlog({
      "GET /media/": () => mediaPage([]),
      "POST /media/": () => json(422, refused),
    });
    renderAt("/media");
    await upload();
    expect(await screen.findByRole("alert")).toHaveTextContent(refused.title);
  });
});
