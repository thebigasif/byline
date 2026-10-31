import { createApi } from "./client";
import { isProblem } from "./posts";
import type { Reply } from "./redirects";
import type { components } from "./schema";

/** The page views of a period, per day, source and page, against the period before. */
export type Visitors = components["schemas"]["AdminVisitors"];
/** The readers each AI assistant sent in a period, against the period before. */
export type AiVisitors = components["schemas"]["AdminAiVisitors"];
/** The pages AI bots read in a period that no AI assistant sent a reader to. */
export type Uncited = components["schemas"]["AdminUncited"];

/** The periods the Analytics screen offers, in days ending today. */
export const PERIODS = [7, 28, 90] as const;

/** One of the periods the Analytics screen offers. */
export type Days = (typeof PERIODS)[number];

/** What an openapi-fetch call gives back. */
type Fetched<T> = { data?: T; error?: unknown; response: Response };

const api = createApi();

/** Turns a call into its data or the problem; a blog that cannot be reached answers status 0. */
async function reply<T>(call: () => Promise<Fetched<T>>): Promise<Reply<T>> {
  try {
    const { data, error, response } = await call();
    if (data !== undefined) {
      return { data };
    }
    return { problem: isProblem(error) ? error : null, status: response.status };
  } catch {
    return { problem: null, status: 0 };
  }
}

/** Reads the page views of the last `days` days. */
export function readVisitors(days: Days): Promise<Reply<Visitors>> {
  return reply(() => api.GET("/api/admin/v1/analytics/visitors/", { params: { query: { days } } }));
}

/** Reads the readers each AI assistant sent in the last `days` days. */
export function readAiVisitors(days: Days): Promise<Reply<AiVisitors>> {
  return reply(() =>
    api.GET("/api/admin/v1/analytics/ai-visitors/", { params: { query: { days } } }),
  );
}

/** Reads the pages AI bots read in the last `days` days but no AI assistant cited. */
export function readUncited(days: Days): Promise<Reply<Uncited>> {
  return reply(() => api.GET("/api/admin/v1/analytics/uncited/", { params: { query: { days } } }));
}
