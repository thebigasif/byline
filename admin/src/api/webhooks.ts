// The webhook routes of the Admin API (task 1.19): list, add, switch off.
import { call, nothing, type Reply, sending } from "./call";
import type { components } from "./schema";

/** A webhook that is on. Its secret is in `secret` only in the answer that added it. */
export type AdminWebhook = components["schemas"]["AdminWebhook"];

/** One event to send to one address. */
export type NewWebhook = components["schemas"]["NewWebhook"];

async function webhooks(response: Response): Promise<AdminWebhook[]> {
  const body: components["schemas"]["AdminWebhooks"] = await response.json();
  return body.webhooks;
}

/** Every webhook that is on, oldest first, never with its secret. */
export function listWebhooks(): Promise<Reply<AdminWebhook[]>> {
  return call("/webhooks/", {}, webhooks);
}

/** Adds a webhook; the answer holds its signing secret this once. */
export function addWebhook(webhook: NewWebhook): Promise<Reply<AdminWebhook[]>> {
  const body: components["schemas"]["NewWebhooks"] = { webhooks: [webhook] };
  return call("/webhooks/", sending("POST", body), webhooks);
}

/** One message a webhook sent: where it stands, and every try. */
export type AdminDelivery = components["schemas"]["AdminDelivery"];

async function deliveries(response: Response): Promise<AdminDelivery[]> {
  const body: components["schemas"]["AdminDeliveries"] = await response.json();
  return body.deliveries;
}

/** The 50 newest messages one webhook that is on sent, newest first. */
export function listDeliveries(id: string): Promise<Reply<AdminDelivery[]>> {
  return call(`/webhooks/${encodeURIComponent(id)}/deliveries/`, {}, deliveries);
}

/** Switches a webhook off; its past deliveries stay in the log. */
export function removeWebhook(id: number): Promise<Reply<null>> {
  return call(`/webhooks/${id}/`, { method: "DELETE" }, nothing);
}
