import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { redirectsBlog } from "../../test/redirects";
import { json, renderAt, signedIn } from "../../test/render";
import type { Passkey } from "../api/passkeys";
import { en } from "../strings/en";

// The options as webauthn-rs sends them: bytes as unpadded base64url.
const OPTIONS = {
  publicKey: {
    rp: { id: "blog.example.com", name: "blog.example.com" },
    user: { id: "AQID", name: "asif@example.com", displayName: "Asif" },
    challenge: "Y2hhbGxlbmdl",
    pubKeyCredParams: [{ type: "public-key", alg: -7 }],
    timeout: 300000,
    excludeCredentials: [{ type: "public-key", id: "BAUG" }],
    attestation: "none",
  },
};

// What the browser's toJSON() gives for the new passkey.
const CREDENTIAL = {
  id: "BwgJ",
  rawId: "BwgJ",
  type: "public-key",
  response: { clientDataJSON: "e30", attestationObject: "oA" },
  clientExtensionResults: {},
};

/** This device, making a passkey as `create` says. */
function device(create: (options: CredentialCreationOptions) => Promise<unknown>) {
  const made = vi.fn(create);
  Object.defineProperty(navigator, "credentials", { value: { create: made }, configurable: true });
  return made;
}

/** A blog that hands out `OPTIONS` and keeps each passkey it is sent. */
function blog() {
  const passkeys: Passkey[] = [];
  return redirectsBlog({
    "GET /session/passkeys/": () => json(200, passkeys),
    "POST /session/passkeys/options/": () => json(200, OPTIONS),
    "POST /session/passkeys/": async (request) => {
      const { name } = await request.clone().json();
      passkeys.push({ id: 1, name, added_at: "2026-10-31T10:00:00Z", last_used_at: null });
      return new Response(null, { status: 201 });
    },
  });
}

/** The requests that added a passkey, not the ones that asked for options. */
function adds(sent: Request[]) {
  const route = (request: Request) => new URL(request.url).pathname;
  return sent.filter((request) => request.method === "POST" && route(request).endsWith("keys/"));
}

describe("adding a passkey", () => {
  it("asks this device for a passkey and sends it to the blog", async () => {
    signedIn();
    const sent = blog();
    const made = { id: "BwgJ", type: "public-key", toJSON: () => CREDENTIAL };
    const create = device(async () => made);
    renderAt("/passkeys");
    await userEvent.type(await screen.findByLabelText(en.passkeys.name), "Work laptop");
    await userEvent.click(screen.getByRole("button", { name: en.passkeys.addText }));
    expect(await screen.findByText(en.passkeys.madeOne("Work laptop"))).toBeVisible();
    expect(await screen.findByRole("cell", { name: "Work laptop" })).toBeVisible();
    const publicKey = create.mock.calls[0]?.[0].publicKey as PublicKeyCredentialCreationOptions;
    expect(new TextDecoder().decode(publicKey.challenge as Uint8Array)).toBe("challenge");
    expect(Array.from(publicKey.user.id as Uint8Array)).toEqual([1, 2, 3]);
    const excluded = publicKey.excludeCredentials?.[0]?.id as Uint8Array;
    expect(Array.from(excluded)).toEqual([4, 5, 6]);
    const body = await adds(sent)[0]?.json();
    const challenge = "Y2hhbGxlbmdl";
    expect(body).toEqual({ name: "Work laptop", challenge, credential: CREDENTIAL });
  });

  it("builds the credential from its buffers in a browser without toJSON", async () => {
    signedIn();
    const sent = blog();
    const bytes = (values: number[]) => new Uint8Array(values).buffer;
    const response = { clientDataJSON: bytes([123, 125]), attestationObject: bytes([160]) };
    device(async () => ({ id: "BwgJ", type: "public-key", rawId: bytes([7, 8, 9]), response }));
    renderAt("/passkeys");
    await userEvent.type(await screen.findByLabelText(en.passkeys.name), "Phone");
    await userEvent.click(screen.getByRole("button", { name: en.passkeys.addText }));
    expect(await screen.findByText(en.passkeys.madeOne("Phone"))).toBeVisible();
    const body = await adds(sent)[0]?.json();
    expect(body.credential).toEqual(CREDENTIAL);
  });

  it("says so when the person cancels, and sends nothing", async () => {
    signedIn();
    const sent = blog();
    device(async () => {
      throw new DOMException("The operation was cancelled.", "NotAllowedError");
    });
    renderAt("/passkeys");
    await userEvent.type(await screen.findByLabelText(en.passkeys.name), "Work laptop");
    await userEvent.click(screen.getByRole("button", { name: en.passkeys.addText }));
    expect(await screen.findByText(en.passkeys.problems.noPasskey.what)).toBeVisible();
    const button = screen.getByRole("button", { name: en.passkeys.addText });
    await waitFor(() => expect(button).toBeEnabled());
    expect(adds(sent)).toHaveLength(0);
  });
});
