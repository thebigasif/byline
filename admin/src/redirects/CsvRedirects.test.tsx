import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { kept, redirectsBlog } from "../../test/redirects";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { StoredRedirect } from "../api/redirects";
import { en } from "../strings/en";

const CSV = "from,to,status,match\n/blog/a/,/blog/b/,301,exact\n";

/** A blog that keeps `stored`, sends it as CSV, and adds an imported list with `answer`. */
function keeping(stored: StoredRedirect[], answer?: () => Response) {
  const list = [...stored];
  return redirectsBlog({
    "GET /not-found/": () => json(200, { misses: [] }),
    "GET /redirects/": () => json(200, { redirects: list }),
    "GET /redirects/export/": () => new Response(CSV, { headers: { "Content-Type": "text/csv" } }),
    "POST /redirects/import/": () => {
      if (answer !== undefined) {
        return answer();
      }
      const added = [kept({ id: 20, from: "/blog/a/", to: "/blog/b/", origin: "import" })];
      list.push(...added);
      return json(201, { redirects: added });
    },
  });
}

/** Makes the browser's file saving watchable, and gives back what it saved; jsdom has none. */
function watchSaves() {
  const saved: Blob[] = [];
  URL.createObjectURL = (made: Blob | MediaSource) => {
    if (made instanceof Blob) {
      saved.push(made);
    }
    return "blob:redirects";
  };
  URL.revokeObjectURL = () => undefined;
  return saved;
}

describe("many redirects at once", () => {
  it("downloads every kept redirect as a CSV file", async () => {
    signedIn();
    keeping([kept()]);
    const saved = watchSaves();
    renderAt("/redirects");
    await userEvent.click(await screen.findByRole("button", { name: en.redirects.csv.download }));
    await vi.waitFor(() => expect(saved).toHaveLength(1));
    expect(await saved[0]?.text()).toBe(CSV);
  });

  it("adds every redirect in an uploaded file and lists them", async () => {
    signedIn();
    const sent = keeping([]);
    renderAt("/redirects");
    const file = new File([CSV], "redirects.csv", { type: "text/csv" });
    await userEvent.upload(await screen.findByLabelText(en.redirects.csv.file), file);
    await userEvent.click(screen.getByRole("button", { name: en.redirects.csv.add }));
    expect(await screen.findByText(en.redirects.csv.added(1))).toBeVisible();
    const posted = sent.find((request) => request.method === "POST");
    expect(await posted?.text()).toBe(CSV);
    const table = await screen.findByRole("table", { name: en.redirects.kept.title });
    expect(table).toHaveTextContent("/blog/a/");
    expect(await axeViolations()).toEqual([]);
  });

  it("shows the API's words when a row cannot be stored", async () => {
    signedIn();
    const unreadable = {
      type: "https://khalid.dev/problems/invalid-body",
      title: "This list of redirects cannot be stored: row 2 has no status.",
      status: 422,
      detail: "Nothing from the list was stored, so no half list sends readers astray.",
      fix: "Fix that row, keep the header from,to,status,match, and send the whole list again.",
    };
    keeping([], () => json(422, unreadable));
    renderAt("/redirects");
    const file = new File([CSV], "redirects.csv", { type: "text/csv" });
    await userEvent.upload(await screen.findByLabelText(en.redirects.csv.file), file);
    await userEvent.click(screen.getByRole("button", { name: en.redirects.csv.add }));
    expect(await screen.findByRole("alert")).toHaveTextContent(unreadable.title);
  });
});
