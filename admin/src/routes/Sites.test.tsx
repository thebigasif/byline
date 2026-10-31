import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { axeViolations, json, renderAt, serve } from "../../test/render";
import { saveCsrf } from "../auth/session";
import { en } from "../strings/en";

const sitesWords = en.sites;

type Answers = Record<string, (request: Request) => Response | Promise<Response>>;

const HOME = { domain: "blog.serpdad.com", home: true };
const NEWS = { domain: "news.serpdad.com", home: false };
const CSRF = "csrf-made-up-for-this-test";

const SESSION = {
  account: { id: 1, email: "asif@serpdad.com", name: "Asif", role: "owner" },
  csrf: CSRF,
};

/** An answer in the API's problem shape. */
function problem(status: number, title: string) {
  return json(status, {
    type: "https://khalid.dev/problems/x",
    title,
    status,
    detail: `Why ${title}`,
    fix: `Fix ${title}`,
  });
}

/** A server that answers Admin API routes from `answers`, and the requests it was sent. */
function server(answers: Answers) {
  const sent: Request[] = [];
  serve((request) => {
    sent.push(request);
    const route = new URL(request.url).pathname.replace("/api/admin/v1", "");
    return answers[`${request.method} ${route}`]?.(request) ?? json(404, {});
  });
  return sent;
}

/** Writes `domain` in the form and presses the add button. */
async function add(domain: string) {
  const field = await screen.findByLabelText(sitesWords.add.label);
  if (domain !== "") {
    await userEvent.type(field, domain);
  }
  await userEvent.click(screen.getByRole("button", { name: sitesWords.add.submit }));
}

describe("the Sites page", () => {
  it("lists the sites and adds one, then says what to do next", async () => {
    saveCsrf(CSRF);
    const sent = server({
      "GET /sites/": () => json(200, { sites: [HOME] }),
      "POST /sites/": () => json(201, NEWS),
    });
    renderAt("/sites");
    const table = await screen.findByRole("table");
    expect(within(table).getByRole("link", { name: HOME.domain })).toBeInTheDocument();
    expect(within(table).getByText(sitesWords.list.home)).toBeInTheDocument();
    expect(await axeViolations()).toEqual([]);

    await add("News.SerpDad.com ");
    expect(await screen.findByText(sitesWords.add.done(NEWS.domain))).toBeInTheDocument();
    expect(screen.getByText(sitesWords.add.dns)).toBeInTheDocument();
    expect(screen.getByText(sitesWords.add.ownerCommand(NEWS.domain))).toBeInTheDocument();
    expect(within(table).getByRole("link", { name: NEWS.domain })).toBeInTheDocument();
    const posted = sent.find((request) => request.method === "POST");
    expect(await posted?.json()).toEqual({ domain: NEWS.domain });
    expect(sent.every((request) => request.headers.get("x-khalid-csrf") === CSRF)).toBe(true);
  });

  it("asks for a sign-in link when the tab has no session, and reads no sites", async () => {
    const sent = server({ "POST /session/link/": () => new Response(null, { status: 202 }) });
    renderAt("/sites");
    const email = SESSION.account.email;
    await userEvent.type(await screen.findByLabelText(sitesWords.signIn.label), email);
    await userEvent.click(screen.getByRole("button", { name: sitesWords.signIn.submit }));
    expect(await screen.findByText(sitesWords.signIn.sent(email))).toBeInTheDocument();
    expect(sent.map((request) => new URL(request.url).pathname)).toEqual([
      "/api/admin/v1/session/link/",
    ]);
    expect(await axeViolations()).toEqual([]);
  });

  it("asks for a sign-in link again when the session has ended", async () => {
    saveCsrf(CSRF);
    server({ "GET /sites/": () => problem(401, "The session has ended.") });
    renderAt("/sites");
    expect(await screen.findByLabelText(sitesWords.signIn.label)).toBeInTheDocument();
  });

  it("shows the API's words to someone the server does not list, and no form", async () => {
    saveCsrf(CSRF);
    server({ "GET /sites/": () => problem(403, "Only a super admin may see the sites.") });
    renderAt("/sites");
    expect(await screen.findByRole("alert")).toHaveTextContent("Only a super admin");
    expect(screen.queryByLabelText(sitesWords.add.label)).toBeNull();
  });

  it("explains an empty domain and a taken one, and adds nothing", async () => {
    saveCsrf(CSRF);
    const sent = server({
      "GET /sites/": () => json(200, { sites: [HOME, NEWS] }),
      "POST /sites/": () => problem(409, "A site already answers on this domain."),
    });
    renderAt("/sites");
    await add("");
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(sitesWords.problems.badDomain.what);
    expect(sent.some((request) => request.method === "POST")).toBe(false);

    await add(NEWS.domain);
    expect(await screen.findByText("A site already answers on this domain.")).toBeInTheDocument();
    expect(screen.queryByText(sitesWords.add.done(NEWS.domain))).toBeNull();
  });
});
