import { type FormEvent, useState } from "react";
import type { NewWebhook } from "../api/webhooks";
import { en } from "../strings/en";
import { EVENT_GROUPS, eventSentence } from "./events";

type Props = { onAdd: (webhook: NewWebhook) => Promise<boolean> };

/** A new webhook: the one event it sends, and the address it sends it to. */
export function NewWebhookForm({ onAdd }: Props) {
  const words = en.webhooks.add;
  const [event, setEvent] = useState("post.published");
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async (submitted: FormEvent<HTMLFormElement>) => {
    submitted.preventDefault();
    setBusy(true);
    const added = await onAdd({ event, url: url.trim() });
    setBusy(false);
    if (added) {
      setUrl("");
    }
  };

  return (
    <form onSubmit={submit}>
      <div className="field">
        <label htmlFor="webhook-event">{words.event}</label>
        <span className="hint" id="webhook-event-hint">
          {words.eventHint}
        </span>
        <select
          id="webhook-event"
          value={event}
          aria-describedby="webhook-event-hint"
          onChange={(changed) => setEvent(changed.target.value)}
        >
          {EVENT_GROUPS.map((group) => (
            <optgroup key={group.label} label={group.label}>
              {group.events.map((name) => (
                <option key={name} value={name}>
                  {eventSentence(name)}
                </option>
              ))}
            </optgroup>
          ))}
        </select>
      </div>
      <div className="field">
        <label htmlFor="webhook-url">{words.url}</label>
        <span className="hint" id="webhook-url-hint">
          {words.urlHint}
        </span>
        <input
          id="webhook-url"
          type="url"
          value={url}
          required
          spellCheck={false}
          aria-describedby="webhook-url-hint"
          onChange={(changed) => setUrl(changed.target.value)}
        />
      </div>
      <p className="muted">{words.once}</p>
      <button type="submit" className="btn primary" disabled={busy}>
        {words.submit}
      </button>
    </form>
  );
}
