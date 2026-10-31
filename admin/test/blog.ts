import { json, serve } from "./render";

/** What the blog answers, keyed by method and route, such as `GET /redirects/`. */
export type Answers = Record<string, (request: Request) => Response | Promise<Response>>;

/** A blog that answers Admin API routes from `answers`, 404 to the rest; gives what it was sent. */
export function adminBlog(answers: Answers) {
  const sent: Request[] = [];
  serve((request) => {
    sent.push(request);
    const route = new URL(request.url).pathname.replace("/api/admin/v1", "");
    return answers[`${request.method} ${route}`]?.(request) ?? json(404, {});
  });
  return sent;
}

/** An answer with no body, such as a delete's. */
export function noContent(): Response {
  return new Response(null, { status: 204 });
}
