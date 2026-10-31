// The staff routes of the Admin API (brief G48): the people who sign in.
import { call, nothing, type Reply, sending } from "./call";
import type { components } from "./schema";

/** One person who signs in to the admin, with their role. */
export type StaffAccount = components["schemas"]["StaffAccount"];

/** A person to invite: they sign in by an emailed link, then pick a password or a passkey. */
export type Invite = components["schemas"]["Invite"];

async function staff(response: Response): Promise<StaffAccount[]> {
  const body: components["schemas"]["StaffList"] = await response.json();
  return body.staff;
}

/** Everyone who signs in; needs a role that manages staff. */
export function listStaff(): Promise<Reply<StaffAccount[]>> {
  return call("/staff/", {}, staff);
}

/** Makes an account with a role below the caller's; sends no email. */
export function inviteStaff(invite: Invite): Promise<Reply<StaffAccount[]>> {
  return call("/staff/", sending("POST", invite), staff);
}

/** Gives an account another role, from their next request. */
export function changeRole(id: number, role: string): Promise<Reply<StaffAccount[]>> {
  const body: components["schemas"]["RoleChange"] = { role };
  return call(`/staff/${id}/`, sending("PUT", body), staff);
}

/** Removes an account, with its sessions and passkeys. */
export function removeStaff(id: number): Promise<Reply<null>> {
  return call(`/staff/${id}/`, { method: "DELETE" }, nothing);
}
