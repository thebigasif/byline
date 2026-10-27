import { describe, expect, it } from "vitest";
import { checkKey } from "./checkKey";

function answering(status: number) {
  const seen: Request[] = [];
  const fetch = async (input: RequestInfo | URL): Promise<Response> => {
    if (input instanceof Request) {
      seen.push(input);
    }
    return new Response("{}", { status, headers: { "Content-Type": "application/json" } });
  };
  return { fetch, seen };
}

describe("checkKey", () => {
  it("accepts a key when the Admin API answers that the probe post does not exist", async () => {
    const server = answering(404);
    expect(await checkKey("ks_0123_abcd", server.fetch)).toBe("accepted");
    expect(server.seen[0]?.headers.get("Authorization")).toBe("Khalid ks_0123_abcd");
    expect(server.seen[0]?.url).toContain("/api/admin/v1/posts/");
  });

  it("refuses a key the server does not accept", async () => {
    expect(await checkKey("wrong", answering(401).fetch)).toBe("rejected");
  });

  it("says when the key may not write posts", async () => {
    expect(await checkKey("reader", answering(403).fetch)).toBe("lacks-scope");
  });

  it("says when the server cannot be reached", async () => {
    const down = async (): Promise<Response> => {
      throw new TypeError("Failed to fetch");
    };
    expect(await checkKey("any", down)).toBe("unreachable");
    expect(await checkKey("any", answering(500).fetch)).toBe("unreachable");
  });
});
