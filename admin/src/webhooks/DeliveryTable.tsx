import type { AdminDelivery } from "../api/webhooks";
import { moment } from "../strings/dates";
import { en } from "../strings/en";
import { DeliveryStands } from "./DeliveryStands";

type Props = { deliveries: AdminDelivery[] };

/** One try in words: the receiver's answer, or why it could not be reached. */
function tried(attempt: AdminDelivery["attempts"][number]): string {
  const words = en.webhooks.sent;
  const at = moment(attempt.at);
  if (attempt.status === undefined || attempt.status === null) {
    return words.unreachable(at, attempt.reason ?? "");
  }
  return words.answered(at, attempt.status);
}

/** When the first try went out, or that none has yet. */
function firstSent(delivery: AdminDelivery): string {
  const first = delivery.attempts[0];
  return first === undefined ? en.webhooks.sent.notYet : moment(first.at);
}

/** What one webhook sent, newest first: when, where it stands, and each try. */
export function DeliveryTable({ deliveries }: Props) {
  const words = en.webhooks.sent;
  if (deliveries.length === 0) {
    return <p>{words.none}</p>;
  }
  return (
    <div className="scroll">
      <table aria-labelledby="sent-title">
        <thead>
          <tr>
            <th scope="col">{words.first}</th>
            <th scope="col">{words.stands}</th>
            <th scope="col">{words.tries}</th>
          </tr>
        </thead>
        <tbody>
          {deliveries.map((delivery) => (
            <tr key={delivery.id}>
              <td>{firstSent(delivery)}</td>
              <td data-label={words.stands}>
                <DeliveryStands delivery={delivery} />
              </td>
              <td data-label={words.tries}>
                <ol>
                  {delivery.attempts.map((attempt) => <li key={attempt.at}>{tried(attempt)}</li>)}
                </ol>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
