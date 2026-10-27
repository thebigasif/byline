import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { kept, miss, redirectsBlog } from "../../test/redirects";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { Miss, StoredRedirect } from "../api/redirects";
import { en } from "../strings/en";

const FAR = miss({ path: "/wp-login.php", hits: 3, nearest: null });
const SEND = en.redirects.misses.send("/blog/internal-links/");

/** A blog whose 404 log holds `misses`, and which redirects one when asked. */
function logging(misses: Miss[], answer = () => json(201, { redirects: [] })) {
  let log = [...misses];
  const stored: StoredRedirect[] = [];
  return redirectsBlog({
    "GET /redirects/": () => json(200, { redirects: stored }),
    "GET /not-found/": () => json(200, { misses: log }),
    "POST /not-found/redirect/": async (request) => {
      // A clone, so the test can still read what was sent.
      const body: { path: string; to: string } = await request.clone().json();
      const reply = answer();
      if (reply.ok) {
        log = log.filter((logged) => logged.path !== body.path);
        stored.push(kept({ from: body.path, to: body.to }));
      }
      return reply;
    },
  });
}

describe("the addresses that found nothing", () => {
  it("lists each with its visits and the page Khalid suggests", async () => {
    signedIn();
    logging([miss(), FAR]);
    renderAt("/redirects");
    const table = await screen.findByRole("table", { name: en.redirects.misses.title });
    const [, near, far] = within(table).getAllByRole("row");
    expect(near).toHaveTextContent("/blog/internal-links/");
    expect(near).toHaveTextContent("42");
    expect(near).toHaveTextContent("/blog/why-internal-links-matter/");
    expect(far).toHaveTextContent(en.redirects.misses.noneNear);
    expect(within(far ?? table).queryByRole("button")).toBeNull();
    expect(await axeViolations()).toEqual([]);
  });

  it("redirects one to the suggested page in one step", async () => {
    signedIn();
    const sent = logging([miss(), FAR]);
    renderAt("/redirects");
    await userEvent.click(await screen.findByRole("button", { name: SEND }));
    const done = en.redirects.misses.sent(miss().path, miss().nearest ?? "");
    expect(await screen.findByText(done)).toBeVisible();
    const posted = sent.find((request) => request.method === "POST");
    expect(await posted?.json()).toEqual({
      path: "/blog/internal-links/",
      to: "/blog/why-internal-links-matter/",
    });
    const log = screen.getByRole("table", { name: en.redirects.misses.title });
    await waitFor(() => expect(within(log).getAllByRole("row")).toHaveLength(2));
    const keptList = await screen.findByRole("table", { name: en.redirects.kept.title });
    expect(keptList).toHaveTextContent("/blog/internal-links/");
  });

  it("shows the API's words when the address already has a redirect", async () => {
    signedIn();
    const taken = {
      type: "https://khalid.dev/problems/conflict",
      title: "The address /blog/internal-links/ already has a redirect.",
      status: 409,
      detail: "One address can lead to only one place.",
      fix: "Remove the redirect it has first, then add the new one.",
    };
    logging([miss()], () => json(409, taken));
    renderAt("/redirects");
    await userEvent.click(await screen.findByRole("button", { name: SEND }));
    expect(await screen.findByRole("alert")).toHaveTextContent(taken.title);
  });

  it("says when no address has found nothing", async () => {
    signedIn();
    logging([]);
    renderAt("/redirects");
    expect(await screen.findByText(en.redirects.misses.none)).toBeVisible();
  });
});
