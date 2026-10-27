import { useQuery } from "@tanstack/react-query";
import { createApi } from "./client";
import { isProblem, type Problem } from "./posts";
import type { components } from "./schema";

/** One row of the posts table: a post in any status, with its checks counted. */
export type PostRow = components["schemas"]["PostRow"];
/** Where a page sits in its list. */
export type Pagination = components["schemas"]["Pagination"];

/** Which rows to send, in the Admin API's list grammar (RULES.md BL-API-5). */
export type RowQuery = { filter?: string; order?: string; limit?: number; page?: number };

/** One page of rows and where it sits in the list. */
export type RowPage = { rows: PostRow[]; pagination: Pagination };

/** A list call that failed: the problem the API sent, if any, and the status. */
export type RowsFailure = { problem: Problem | null; status: number };

/** What a list call gave back: a page, or the problem. */
export type Rows = RowPage | RowsFailure;

const api = createApi();

/** Whether a list call gave back a page. */
export function isPage(rows: Rows | undefined): rows is RowPage {
  return rows !== undefined && "rows" in rows;
}

/** Whether a list call has answered, with a problem. */
export function isFailure(rows: Rows | undefined): rows is RowsFailure {
  return rows !== undefined && !isPage(rows);
}

/** Lists posts in every status; a server that cannot be reached answers status 0. */
export async function listPosts(query: RowQuery): Promise<Rows> {
  try {
    const { data, error, response } = await api.GET("/api/admin/v1/posts/", {
      params: {
        query: {
          filter: query.filter,
          order: query.order,
          limit: query.limit?.toString(),
          page: query.page?.toString(),
        },
      },
    });
    if (data !== undefined) {
      return { rows: data.posts, pagination: data.meta.pagination };
    }
    return { problem: isProblem(error) ? error : null, status: response.status };
  } catch {
    return { problem: null, status: 0 };
  }
}

/** One page of rows, kept by TanStack Query; `undefined` until it arrives. */
export function useRows(query: RowQuery): Rows | undefined {
  return useQuery({ queryKey: ["rows", query], queryFn: () => listPosts(query) }).data;
}
