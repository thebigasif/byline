import { Link } from "@tanstack/react-router";
import type { AdminWebhook } from "../api/webhooks";
import { day } from "../strings/dates";
import { en } from "../strings/en";
import { eventSentence, hostOf } from "./events";

type Props = { webhooks: AdminWebhook[]; onRemove: (webhook: AdminWebhook) => void };

/** The webhooks that are on, each with its event in words and a way to switch it off. */
export function WebhookTable({ webhooks, onRemove }: Props) {
  const words = en.webhooks.list;
  if (webhooks.length === 0) {
    return <p>{words.none}</p>;
  }
  return (
    <div className="scroll">
      <table aria-labelledby="webhooks-title">
        <thead>
          <tr>
            <th scope="col">{words.event}</th>
            <th scope="col">{words.url}</th>
            <th scope="col">{words.added}</th>
            <th scope="col">
              <span className="muted">{words.action}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {webhooks.map((webhook) => (
            <tr key={webhook.id}>
              <td>
                {eventSentence(webhook.event)} <code>{webhook.event}</code>
              </td>
              <td data-label={words.url}>{webhook.url}</td>
              <td data-label={words.added}>{day(webhook.created_at)}</td>
              <td>
                <Link
                  to="/webhooks/$id"
                  params={{ id: String(webhook.id) }}
                  className="btn"
                  aria-label={words.seeLabel(eventSentence(webhook.event), hostOf(webhook.url))}
                >
                  {words.see}
                </Link>{" "}
                <button
                  type="button"
                  className="btn"
                  aria-label={words.removeLabel(eventSentence(webhook.event), hostOf(webhook.url))}
                  onClick={() => onRemove(webhook)}
                >
                  {words.remove}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
