import type { AdminAuthor, AuthorEdit } from "../api/authors";

/** An author's fields as the form holds them: plain text, profiles one per line. */
export type AuthorFields = {
  name: string;
  role: string;
  credentials: string;
  photo: string;
  bio: string;
  sameAs: string;
};

/** The form's starting text for `author`. */
export function fieldsOf(author: AdminAuthor): AuthorFields {
  return {
    name: author.name,
    role: author.role ?? "",
    credentials: author.credentials ?? "",
    photo: author.photo ?? "",
    bio: author.bio,
    sameAs: author.same_as.join("\n"),
  };
}

function profiles(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

/**
 * The edit that turns `author` into `fields`, with only the fields that
 * changed; `null` when nothing did. An empty role, credentials or photo
 * removes it, as the API reads it.
 */
export function changes(author: AdminAuthor, fields: AuthorFields): AuthorEdit | null {
  const before = fieldsOf(author);
  const edit: AuthorEdit = { updated_at: author.updated_at };
  let changed = false;
  for (const key of ["name", "role", "credentials", "photo", "bio"] as const) {
    if (fields[key].trim() !== before[key].trim()) {
      edit[key] = fields[key].trim();
      changed = true;
    }
  }
  const sameAs = profiles(fields.sameAs);
  if (sameAs.join("\n") !== author.same_as.join("\n")) {
    edit.same_as = sameAs;
    changed = true;
  }
  return changed ? edit : null;
}
