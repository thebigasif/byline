// The browser side of adding a passkey. The API sends and reads bytes as
// unpadded base64url; the browser's WebAuthn calls take and give buffers.
import type { components } from "../api/schema";

/** What `navigator.credentials.create` gets, as the API sends it. */
export type CreateOptions = components["schemas"]["CreateOptions"];

/** A passkey the browser made, ready to send to the API. */
export type MadePasskey = { client_data: string; attestation_object: string };

/** Bytes from unpadded base64url. */
export function fromBase64Url(text: string) {
  const base64 = text.replaceAll("-", "+").replaceAll("_", "/");
  const binary = atob(base64.padEnd(Math.ceil(base64.length / 4) * 4, "="));
  return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

/** Bytes as unpadded base64url. */
export function toBase64Url(bytes: ArrayBuffer): string {
  const binary = Array.from(new Uint8Array(bytes), (byte) => String.fromCharCode(byte)).join("");
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

type Attestation = { clientDataJSON: ArrayBuffer; attestationObject: ArrayBuffer };

// Checked by shape, because a test browser has no PublicKeyCredential class.
function attestation(made: Credential | null): Attestation | null {
  if (made === null || !("response" in made)) {
    return null;
  }
  const response = made.response;
  if (
    typeof response === "object" &&
    response !== null &&
    "clientDataJSON" in response &&
    response.clientDataJSON instanceof ArrayBuffer &&
    "attestationObject" in response &&
    response.attestationObject instanceof ArrayBuffer
  ) {
    return {
      clientDataJSON: response.clientDataJSON,
      attestationObject: response.attestationObject,
    };
  }
  return null;
}

/**
 * Asks the browser to make a passkey for `email`, shown as `name` in the
 * device's list; `null` when the person cancels or the browser cannot.
 */
export async function makePasskey(
  options: CreateOptions,
  email: string,
  name: string,
): Promise<MadePasskey | null> {
  const type = "public-key" as const;
  const publicKey: PublicKeyCredentialCreationOptions = {
    challenge: fromBase64Url(options.challenge),
    rp: { id: options.rp_id, name: options.rp_id },
    user: { id: fromBase64Url(options.user_id), name: email, displayName: name },
    pubKeyCredParams: options.algorithms.map((alg) => ({ type, alg })),
    excludeCredentials: options.exclude.map((id) => ({ type, id: fromBase64Url(id) })),
    timeout: options.timeout_ms,
    authenticatorSelection: { residentKey: "preferred", userVerification: "required" },
    attestation: "none",
  };
  try {
    const made = attestation(await navigator.credentials.create({ publicKey }));
    if (made === null) {
      return null;
    }
    return {
      client_data: toBase64Url(made.clientDataJSON),
      attestation_object: toBase64Url(made.attestationObject),
    };
  } catch {
    return null;
  }
}
