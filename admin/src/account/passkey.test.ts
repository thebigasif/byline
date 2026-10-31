import { describe, expect, it } from "vitest";
import { fromBase64Url, toBase64Url } from "./passkey";

describe("bytes as the passkey routes write them", () => {
  it("reads unpadded base64url, with - and _", () => {
    expect(Array.from(fromBase64Url("-_8"))).toEqual([251, 255]);
    expect(Array.from(fromBase64Url("AQID"))).toEqual([1, 2, 3]);
  });

  it("writes it back without padding", () => {
    expect(toBase64Url(new Uint8Array([251, 255]).buffer)).toBe("-_8");
    expect(toBase64Url(new Uint8Array([1, 2, 3, 4]).buffer)).toBe("AQIDBA");
  });
});
