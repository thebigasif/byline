import { describe, expect, it } from "vitest";
import { json, serve, signedIn } from "../../test/render";
import { readCsrf, saveCsrf } from "../auth/session";
import { addSite, listSites, redeemLink, sendLink } from "./sites";

/** A blog that answers every call with `answer`, and the requests it was sent. */
function blog(answer: () => Response) {
  const sent: Request[] = [];
  serve((request) => {
    sent.push(request);
    return answer();
  });
  return sent;
}

const HOME = { domain: "blog.serpdad.com", home: true };
const NEWS = { domain: "news.serpdad.com", home: false };

const SESSION = {
  account: { id: 1, email: "asif@serpdad.com", name: "Asif", role: "owner" },
  csrf: "csrf-made-up-for-this-test",
};

const NOT_LISTED = {
  type: "https://khalid.dev/problems/forbidden",
  title: "Only a super admin may see and add the sites on this server.",
  status: 403,
  detail: "Adding a site gives it a domain on this server, so only the people the server lists may.",
  fix: "Sign in on the home site with a super-admin account, or ask the server's owner to add you.",
};

describe("the sites client", () => {
  it("lists the sites with the session's CSRF token and never a key", async () => {
    signedIn();
    saveCsrf(SESSION.csrf);
    const sent = blog(() => json(200, { sites: [HOME, NEWS] }));
    expect(await listSites()).toEqual({ data: [HOME, NEWS] });
    const request = sent[0];
    expect(new URL(request?.url ?? "").pathname).toBe("/api/admin/v1/sites/");
    expect(request?.headers.get("x-khalid-csrf")).toBe(SESSION.csrf);
    expect(request?.headers.get("Authorization")).toBeNull();
  });

  it("adds a site by its domain and reads back the site made", async () => {
    saveCsrf(SESSION.csrf);
    const sent = blog(() => json(201, NEWS));
    expect(await addSite("news.serpdad.com")).toEqual({ data: NEWS });
    expect(sent[0]?.method).toBe("POST");
    expect(sent[0]?.headers.get("Content-Type")).toBe("application/json");
    expect(await sent[0]?.json()).toEqual({ domain: "news.serpdad.com" });
  });

  it("asks for a sign-in link by email", async () => {
    const sent = blog(() => new Response(null, { status: 202 }));
    expect(await sendLink("asif@serpdad.com")).toEqual({ data: null });
    expect(new URL(sent[0]?.url ?? "").pathname).toBe("/api/admin/v1/session/link/");
    expect(await sent[0]?.json()).toEqual({ email: "asif@serpdad.com" });
  });

  it("keeps the CSRF token of a redeemed link for this tab", async () => {
    const sent = blog(() => json(200, SESSION));
    expect(await redeemLink("lt_made_up")).toEqual({ data: SESSION });
    expect(new URL(sent[0]?.url ?? "").pathname).toBe("/api/admin/v1/session/link/redeem/");
    expect(await sent[0]?.json()).toEqual({ token: "lt_made_up" });
    expect(readCsrf()).toBe(SESSION.csrf);
  });

  it("keeps nothing when a link does not sign in", async () => {
    blog(() => json(401, { ...NOT_LISTED, status: 401 }));
    await redeemLink("lt_used");
    expect(readCsrf()).toBeNull();
  });

  it("hands back the problem the API sent, and its status", async () => {
    saveCsrf(SESSION.csrf);
    blog(() => json(403, NOT_LISTED));
    expect(await listSites()).toEqual({ problem: NOT_LISTED, status: 403 });
  });

  it("answers status 0 with no problem when the blog cannot be reached", async () => {
    serve(() => {
      throw new TypeError("Failed to fetch");
    });
    expect(await addSite("news.serpdad.com")).toEqual({ problem: null, status: 0 });
  });
});
