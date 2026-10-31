import { en } from "../strings/en";

/** The roles anyone may be given, in the invite form's order; only a hand-over makes an owner. */
export const GIVEN = ["writer", "contributor", "editor", "admin"] as const;

// Looked up by the names the API sends, which may grow.
const ROLES: Readonly<Record<string, string>> = en.team.roles;

/** A role as the admin names it, such as "Editor"; an unknown one by its API name. */
export function roleName(role: string): string {
  return ROLES[role] ?? role;
}
