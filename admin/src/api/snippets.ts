// The snippet routes of the Admin API (G24). They are typed by hand, field
// for field as `AdminSnippet` and `NewSnippet`, because the OpenAPI document
// gains them only when the G24 stack lands. Then these types become
// `components["schemas"]` and the calls go through `createApi`.
import { useQuery } from "@tanstack/react-query";
import { call, type Reply } from "./call";

/** A snippet in one language, as the Admin API sends it. */
export type Snippet = {
  /** The language and the slug, such as `en.affiliate-disclosure`. */
  id: string;
  /** The slug a post places it by. */
  slug: string;
  language: string;
  /** The name shown in the editor. */
  title: string;
  /** The blocks, in markdown. */
  markdown: string;
  updated_at: string;
};

/** A new snippet, in the site's default language. */
export type NewSnippet = { slug: string; title: string; markdown: string };

async function snippets(response: Response): Promise<Snippet[]> {
  const body: { snippets: Snippet[] } = await response.json();
  return body.snippets;
}

/** Every snippet in every language. */
export function listSnippets(): Promise<Reply<Snippet[]>> {
  return call("/snippets/", {}, snippets);
}

/** Saves a new snippet; gives back the one made. */
export function createSnippet(snippet: NewSnippet): Promise<Reply<Snippet[]>> {
  const init = {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ snippets: [snippet] }),
  };
  return call("/snippets/", init, snippets);
}

/** The site's snippets, once loaded. */
export function useSnippets(): Reply<Snippet[]> | undefined {
  return useQuery({ queryKey: ["snippets"], queryFn: listSnippets }).data;
}

/** The slug a snippet called `name` gets: small letters, digits and hyphens. */
export function snippetSlug(name: string): string {
  return name
    .normalize("NFKD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}
