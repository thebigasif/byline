import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { adminBlog } from "../../test/blog";
import { axeViolations, json, renderAt, signedIn } from "../../test/render";
import { saveCsrf } from "../auth/session";
import { en } from "../strings/en";

const LENA = { account: { id: 1, email: "lena@example.com", name: "Lena", role: "editor" } };

// What the API sends to start a passkey; each value is base64url.
const OPTIONS = {
  challenge: "AQID",
  rp_id: "localhost",
  user_id: "BAUG",
  exclude: ["BwgJ"],
  algorithms: [-7, -8],
  timeout_ms: 300000,
};

/** Gives this test browser a passkey maker that answers with `make`. */
function browserMakes(make: () => Promise<unknown>) {
  const create = vi.fn(make);
  Object.defineProperty(navigator, "credentials", { value: { create }, configurable: true });
  return create;
}

/** A device's answer: the client data and attestation as buffers. */
function attested() {
  return {
    response: {
      clientDataJSON: new Uint8Array([251, 255]).buffer,
      attestationObject: new Uint8Array([1, 2, 3]).buffer,
    },
  };
}

/** A blog with Lena's session that adds every passkey it is sent. */
function blog() {
  const added: unknown[] = [];
  const sent = adminBlog({
    "GET /session/": () => json(200, { ...LENA, csrf: "csrf-test" }),
    "POST /session/passkeys/options/": () => json(200, OPTIONS),
    "POST /session/passkeys/": async (request) => {
      added.push(await request.json());
      return new Response(null, { status: 201 });
    },
  });
  return { sent, added };
}

afterEach(() => {
  Reflect.deleteProperty(navigator, "credentials");
});

describe("adding a passkey", () => {
  it("makes one on the device and sends it to the API as base64url", async () => {
    saveCsrf("csrf-test");
    const { sent, added } = blog();
    const create = browserMakes(async () => attested());
    renderAt("/");
    await userEvent.click(await screen.findByRole("link", { name: en.account.nav }));
    expect(await screen.findByText(en.account.lede("Lena", "lena@example.com"))).toBeVisible();
    await userEvent.type(screen.getByLabelText(en.account.passkey.name), "Work laptop");
    await userEvent.click(screen.getByRole("button", { name: en.account.passkey.submit }));
    expect(await screen.findByText(en.account.passkey.added("Work laptop"))).toBeVisible();
    expect(added).toEqual([
      { name: "Work laptop", client_data: "-_8", attestation_object: "AQID" },
    ]);
    expect(JSON.stringify(create.mock.calls[0])).toContain('"name":"lena@example.com"');
    const signed = sent.every((request) => request.headers.get("x-khalid-csrf") === "csrf-test");
    expect(signed).toBe(true);
    expect(await axeViolations()).toEqual([]);
  });

  it("explains a passkey the device did not make, and sends nothing", async () => {
    saveCsrf("csrf-test");
    const { added } = blog();
    browserMakes(async () => {
      throw new DOMException("Cancelled", "NotAllowedError");
    });
    renderAt("/account");
    await userEvent.type(await screen.findByLabelText(en.account.passkey.name), "Phone");
    await userEvent.click(screen.getByRole("button", { name: en.account.passkey.submit }));
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.noPasskey.what);
    expect(added).toEqual([]);
  });

  it("explains that a staff token has no account", async () => {
    signedIn();
    adminBlog({ "GET /session/": () => json(401, {}) });
    renderAt("/account");
    expect(await screen.findByRole("alert")).toHaveTextContent(en.problems.noSession.what);
    expect(screen.queryByLabelText(en.account.passkey.name)).toBeNull();
  });
});
