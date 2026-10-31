import { createApi } from "./client";
import { isProblem, type Problem } from "./posts";
import type { components } from "./schema";

/** The site's schema templates, the collections to pick from, and whether a restart is due. */
export type SchemaTemplates = components["schemas"]["SchemaTemplates"];
/** One schema.org node for every post in a collection. */
export type SchemaTemplate = components["schemas"]["AdminSchemaTemplate"];

/** What a schema templates call gave back: the templates, or the problem. */
export type TemplatesAnswer =
  | { templates: SchemaTemplates }
  | { problem: Problem | null; status: number };

const api = createApi();

function answer(data: SchemaTemplates | undefined, error: unknown, status: number) {
  if (data !== undefined) {
    return { templates: data };
  }
  return { problem: isProblem(error) ? error : null, status };
}

const UNREACHABLE: TemplatesAnswer = { problem: null, status: 0 };

/** Reads the site's schema templates. */
export async function readTemplates(): Promise<TemplatesAnswer> {
  const sent = await api.GET("/api/admin/v1/schema-templates/").catch(() => null);
  return sent === null ? UNREACHABLE : answer(sent.data, sent.error, sent.response.status);
}

/** Puts `templates` in place of all of them, on the copy last changed at `updatedAt`. */
export async function saveTemplates(
  templates: SchemaTemplate[],
  updatedAt: string,
): Promise<TemplatesAnswer> {
  const body = { templates, updated_at: updatedAt };
  const sent = await api.PUT("/api/admin/v1/schema-templates/", { body }).catch(() => null);
  return sent === null ? UNREACHABLE : answer(sent.data, sent.error, sent.response.status);
}
