import type { AdminDelivery } from "../api/webhooks";
import { moment } from "../strings/dates";
import { en } from "../strings/en";

type Props = { delivery: AdminDelivery };

/** Where one delivery stands, in words, with a hint when there is nothing left to try. */
export function DeliveryStands({ delivery }: Props) {
  const words = en.webhooks.sent;
  const at = moment(delivery.state_at);
  switch (delivery.state) {
    case "delivered":
      return <span className="pill ok">{words.delivered}</span>;
    case "pending":
      return (
        <span className="pill warn">
          {delivery.attempts.length === 0 ? words.sending(at) : words.retrying(at)}
        </span>
      );
    case "gave_up":
      return (
        <>
          <span className="pill block">{words.gaveUp}</span>
          <span className="hint">{words.gaveUpHint}</span>
        </>
      );
    default:
      return <span className="pill quiet">{words.unknown}</span>;
  }
}
