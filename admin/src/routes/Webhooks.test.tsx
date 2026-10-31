import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { adminBlog, noContent } from "../../test/blog";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import type { AdminWebhook } from "../api/webhooks";
import { en } from "../strings/en";

/** The two webhooks from the mockup, as the API lists them: never with a secret. */
const KEPT: AdminWebhook[] = [
  {
    id: 3,
    event: "post.published",
    url: "https://hooks.zapier.com/hooks/catch/123/abc/",
    created_at: "2026-10-28T09:00:00Z",
  },
  {
    id: 4,
    event: "comment.added",
    url: "https://hooks.slack.com/services/T000/B000/XXXX",
    created_at: "2026-10-29T09:00:00Z",
  },
];

const words = en.webhooks;

describe("the Webhooks screen", () => {
  it("lists each webhook with its event in words", async () => {
    signedIn();
    adminBlog({ "GET /webhooks/": () => json(200, { webhooks: KEPT }) });
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.nav.webhooks }));
    const table = await screen.findByRole("table", { name: words.list.title });
    const [, zapier, slack] = within(table).getAllByRole("row");
    expect(zapier).toHaveTextContent("A post goes live");
    expect(zapier).toHaveTextContent("hooks.zapier.com");
    expect(slack).toHaveTextContent("A reader writes a comment");
    expect(await axeViolations()).toEqual([]);
  });

  it("says so when there are none yet", async () => {
    signedIn();
    adminBlog({ "GET /webhooks/": () => json(200, { webhooks: [] }) });
    renderAt("/webhooks");
    expect(await screen.findByText(words.list.none)).toBeVisible();
  });

  it("adds a webhook for the event picked, and shows its secret once", async () => {
    signedIn();
    const asked: unknown[] = [];
    // Made up here; a real secret is never in a test.
    const secret = "whsec_test_only_0123";
    const url = "https://hooks.example.com/khalid";
    const made = { id: 5, event: "comment.added", url, created_at: "2026-10-31T09:00:00Z" };
    adminBlog({
      "GET /webhooks/": () => json(200, { webhooks: KEPT }),
      "POST /webhooks/": async (request) => {
        asked.push(await request.json());
        return json(201, { webhooks: [{ ...made, secret }] });
      },
    });
    const { unmount } = renderAt("/webhooks");
    const event = await screen.findByLabelText(words.add.event);
    await userEvent.selectOptions(event, "A reader writes a comment");
    await userEvent.type(screen.getByLabelText(words.add.url), url);
    await userEvent.click(screen.getByRole("button", { name: words.add.submit }));
    expect(await screen.findByLabelText(words.shown.label)).toHaveValue(secret);
    expect(asked).toEqual([{ webhooks: [{ event: "comment.added", url }] }]);
    expect(await axeViolations()).toEqual([]);
    unmount();
    renderAt("/webhooks");
    expect(await screen.findByRole("table", { name: words.list.title })).toBeVisible();
    expect(screen.queryByDisplayValue(secret)).toBeNull();
  });

  it("shows the API's words when it refuses an address", async () => {
    signedIn();
    const refused = {
      type: "https://khalid.dev/problems/invalid-body",
      title: "The url is not an address Khalid can send to.",
      status: 422,
      detail: "Deliveries go over the web, so the address needs no user name or password.",
      fix: "Give the full address, such as https://hooks.example.com/khalid.",
    };
    adminBlog({
      "GET /webhooks/": () => json(200, { webhooks: KEPT }),
      "POST /webhooks/": () => json(422, refused),
    });
    renderAt("/webhooks");
    const address = await screen.findByLabelText(words.add.url);
    await userEvent.type(address, "https://me:pw@hooks.example.com/khalid");
    await userEvent.click(screen.getByRole("button", { name: words.add.submit }));
    expect(await screen.findByRole("alert")).toHaveTextContent(refused.title);
    expect(screen.queryByLabelText(words.shown.label)).toBeNull();
    expect(address).toHaveValue("https://me:pw@hooks.example.com/khalid");
  });

  it("switches a webhook off, and it leaves the list", async () => {
    signedIn();
    let list = [...KEPT];
    const sent = adminBlog({
      "GET /webhooks/": () => json(200, { webhooks: list }),
      "DELETE /webhooks/3/": () => {
        list = list.filter((webhook) => webhook.id !== 3);
        return noContent();
      },
    });
    renderAt("/webhooks");
    const remove = words.list.removeLabel("A post goes live", "hooks.zapier.com");
    await userEvent.click(await screen.findByRole("button", { name: remove }));
    const removed = words.list.removed("A post goes live", "hooks.zapier.com");
    expect(await screen.findByText(removed)).toBeVisible();
    await waitFor(() => expect(screen.queryByText(/hooks\.zapier\.com\/hooks/)).toBeNull());
    expect(sent.filter((request) => request.method === "DELETE")).toHaveLength(1);
  });
});
