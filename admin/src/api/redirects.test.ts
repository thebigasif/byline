import { describe, expect, it } from "vitest";
import { kept, miss } from "../../test/redirects";
import { json, serve, signedIn } from "../../test/render";
import {
  exportRedirects,
  importRedirects,
  listMisses,
  listRedirects,
  redirectMiss,
  removeRedirect,
} from "./redirects";

/** A blog that answers every call with `answer`, and the requests it was sent. */
function blog(answer: () => Response) {
  const sent: Request[] = [];
  serve((request) => {
    sent.push(request);
    return answer();
  });
  return sent;
}

const TAKEN = {
  type: "https://khalid.dev/problems/conflict",
  title: "The address /blog/a/ already has a redirect.",
  status: 409,
  detail: "One address can lead to only one place.",
  fix: "Remove the redirect it has first, then add the new one.",
};

describe("the redirects client", () => {
  it("lists the redirects Khalid keeps with the signed-in key", async () => {
    signedIn();
    const sent = blog(() => json(200, { redirects: [kept()] }));
    expect(await listRedirects()).toEqual({ data: [kept()] });
    const request = sent[0];
    expect(new URL(request?.url ?? "").pathname).toBe("/api/admin/v1/redirects/");
    expect(request?.headers.get("Authorization")).toBe("Khalid ks_0123456789abcdef_test");
  });

  it("removes one redirect by its identity", async () => {
    const sent = blog(() => new Response(null, { status: 204 }));
    expect(await removeRedirect(12)).toEqual({ data: null });
    expect(sent[0]?.method).toBe("DELETE");
    expect(new URL(sent[0]?.url ?? "").pathname).toBe("/api/admin/v1/redirects/12/");
  });

  it("sends a CSV list as text and reads back what was stored", async () => {
    const sent = blog(() => json(201, { redirects: [kept({ origin: "import" })] }));
    const csv = "from,to,status\n/blog/a/,/blog/b/,301\n";
    expect(await importRedirects(csv)).toEqual({ data: [kept({ origin: "import" })] });
    expect(sent[0]?.method).toBe("POST");
    expect(sent[0]?.headers.get("Content-Type")).toBe("text/csv");
    expect(await sent[0]?.text()).toBe(csv);
  });

  it("reads every redirect back as CSV text", async () => {
    blog(() => new Response("from,to,status,match\n", { headers: { "Content-Type": "text/csv" } }));
    expect(await exportRedirects()).toEqual({ data: "from,to,status,match\n" });
  });

  it("lists the 404 log and redirects one address to the page given", async () => {
    const sent = blog(() => json(200, { misses: [miss()] }));
    expect(await listMisses()).toEqual({ data: [miss()] });
    expect(new URL(sent[0]?.url ?? "").pathname).toBe("/api/admin/v1/not-found/");
    sent.length = 0;
    serve((request) => {
      sent.push(request);
      return json(201, { redirects: [kept()] });
    });
    expect(await redirectMiss("/blog/internal-links/", "/blog/why/")).toEqual({ data: [kept()] });
    expect(await sent[0]?.json()).toEqual({ path: "/blog/internal-links/", to: "/blog/why/" });
  });

  it("hands back the problem the API sent, and its status", async () => {
    blog(() => json(409, TAKEN));
    expect(await redirectMiss("/blog/a/", "/blog/b/")).toEqual({ problem: TAKEN, status: 409 });
  });

  it("answers status 0 with no problem when the blog cannot be reached", async () => {
    serve(() => {
      throw new TypeError("Failed to fetch");
    });
    expect(await listRedirects()).toEqual({ problem: null, status: 0 });
  });
});
