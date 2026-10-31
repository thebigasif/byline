import type { AdminWebhook } from "../api/webhooks";
import { en } from "../strings/en";
import { eventSentence, hostOf } from "./events";

type Props = { added: AdminWebhook };

/** A new webhook's signing secret, shown this once in a field to copy from. */
export function SecretOnce({ added }: Props) {
  const words = en.webhooks.shown;
  return (
    <div className="card" role="status">
      <p>
        <strong>{words.title(eventSentence(added.event), hostOf(added.url))}</strong>
      </p>
      <div className="field">
        <label htmlFor="webhook-secret">{words.label}</label>
        <span className="hint" id="webhook-secret-hint">
          {words.hint}
        </span>
        <input
          id="webhook-secret"
          readOnly
          spellCheck={false}
          aria-describedby="webhook-secret-hint"
          value={added.secret ?? ""}
          onFocus={(focused) => focused.target.select()}
        />
      </div>
    </div>
  );
}
