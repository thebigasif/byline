// The author routes of the Admin API. They are typed by hand, field for
// field as `AdminAuthor`, `NewAuthor` and `AuthorEdit` in the G24 stack's
// OpenAPI document, because this document gains them only when that stack
// lands (docs/decisions/2026-10-31-admin-screens.md). Then these types
// become `components["schemas"]`.
import { call, nothing, type Reply, sending } from "./call";

/** An author in one language, as their file in the site folder holds them. */
export type AdminAuthor = {
  /** The language and the slug, such as `en.asif`. */
  id: string;
  /** The short name posts name the author by. */
  slug: string;
  language: string;
  name: string;
  role?: string | null;
  /** Why to trust the author, such as ten years in the field. */
  credentials?: string | null;
  photo?: string | null;
  /** The bio, in markdown. */
  bio: string;
  /** Profiles elsewhere, each a full address, in order. */
  same_as: string[];
  /** When the author's file last changed; an edit or delete sends it back. */
  updated_at: string;
};

/** A new author; the language is the site's default when left out. */
export type NewAuthor = {
  slug: string;
  name: string;
  language?: string | null;
  role?: string | null;
  bio?: string;
};

/** A change to an author: only the fields sent change; an empty role removes it. */
export type AuthorEdit = {
  updated_at: string;
  name?: string | null;
  role?: string | null;
  credentials?: string | null;
  photo?: string | null;
  bio?: string | null;
  same_as?: string[] | null;
};

async function authors(response: Response): Promise<AdminAuthor[]> {
  const body: { authors: AdminAuthor[] } = await response.json();
  return body.authors;
}

/** Every author in every language. */
export function listAuthors(): Promise<Reply<AdminAuthor[]>> {
  return call("/authors/", {}, authors);
}

/** One author, by language and slug. */
export function readAuthor(id: string): Promise<Reply<AdminAuthor[]>> {
  return call(`/authors/${encodeURIComponent(id)}/`, {}, authors);
}

/** Makes an author's file. */
export function createAuthor(author: NewAuthor): Promise<Reply<AdminAuthor[]>> {
  return call("/authors/", sending("POST", { authors: [author] }), authors);
}

/** Saves an edit made on the copy that `edit.updated_at` names. */
export function editAuthor(id: string, edit: AuthorEdit): Promise<Reply<AdminAuthor[]>> {
  return call(`/authors/${encodeURIComponent(id)}/`, sending("PUT", { authors: [edit] }), authors);
}

/** Deletes an author's file, unless it changed after the copy `updatedAt` names. */
export function deleteAuthor(id: string, updatedAt: string): Promise<Reply<null>> {
  const query = new URLSearchParams({ updated_at: updatedAt });
  return call(`/authors/${encodeURIComponent(id)}/?${query}`, { method: "DELETE" }, nothing);
}
