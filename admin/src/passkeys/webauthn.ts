// The browser side of adding a passkey. The API sends the options in the
// standard WebAuthn JSON, bytes as unpadded base64url, and takes back the
// credential's toJSON(). Newer browsers convert both ways themselves; for
// older ones the few byte fields are converted here.

type Json = Record<string, unknown>;

/** What the API needs back: the challenge the options carried, and the credential. */
export type MadePasskey = { challenge: string; credential: Json };

function isJson(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Bytes from unpadded base64url. */
export function fromBase64Url(text: string): Uint8Array<ArrayBuffer> {
  const base64 = text.replaceAll("-", "+").replaceAll("_", "/");
  const binary = atob(base64.padEnd(Math.ceil(base64.length / 4) * 4, "="));
  return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

/** Bytes as unpadded base64url. */
export function toBase64Url(bytes: ArrayBuffer): string {
  const binary = Array.from(new Uint8Array(bytes), (byte) => String.fromCharCode(byte)).join("");
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

/** The options with their byte fields as bytes, for `navigator.credentials.create`. */
function creationOptions(json: Json): PublicKeyCredentialCreationOptions {
  if (
    "PublicKeyCredential" in globalThis &&
    typeof PublicKeyCredential.parseCreationOptionsFromJSON === "function"
  ) {
    const standard = json as unknown as PublicKeyCredentialCreationOptionsJSON;
    return PublicKeyCredential.parseCreationOptionsFromJSON(standard);
  }
  const user = isJson(json.user) ? json.user : {};
  const exclude = Array.isArray(json.excludeCredentials) ? json.excludeCredentials : [];
  const options = {
    ...json,
    challenge: fromBase64Url(String(json.challenge)),
    user: { ...user, id: fromBase64Url(String(user.id)) },
    excludeCredentials: exclude.filter(isJson).map((credential) => ({
      ...credential,
      id: fromBase64Url(String(credential.id)),
    })),
  };
  return options as unknown as PublicKeyCredentialCreationOptions;
}

/** The credential as its own toJSON() gives it, or built the same way from its buffers. */
function credentialJson(made: Credential): Json | null {
  if ("toJSON" in made && typeof made.toJSON === "function") {
    const json: unknown = made.toJSON();
    return isJson(json) ? json : null;
  }
  if (!("rawId" in made && made.rawId instanceof ArrayBuffer && "response" in made)) {
    return null;
  }
  const response = made.response as AuthenticatorAttestationResponse;
  return {
    id: made.id,
    rawId: toBase64Url(made.rawId),
    type: made.type,
    response: {
      clientDataJSON: toBase64Url(response.clientDataJSON),
      attestationObject: toBase64Url(response.attestationObject),
    },
    clientExtensionResults: {},
  };
}

/**
 * Asks the browser to make a passkey from `options`, as the API sent them.
 * `null` when the person cancels, or the browser cannot make one.
 */
export async function makePasskey(options: unknown): Promise<MadePasskey | null> {
  const publicKey = isJson(options) ? options.publicKey : null;
  if (!isJson(publicKey) || typeof publicKey.challenge !== "string") {
    return null;
  }
  try {
    const made = await navigator.credentials.create({ publicKey: creationOptions(publicKey) });
    const credential = made === null ? null : credentialJson(made);
    return credential === null ? null : { challenge: publicKey.challenge, credential };
  } catch {
    return null;
  }
}
