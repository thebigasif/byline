import { type FormEvent, useState } from "react";
import { en } from "../strings/en";

type Props = { id: number; onSend: (name: string, text: string) => Promise<boolean> };

/** A staff reply to one comment, folded away until it is opened. */
export function ReplyForm({ id, onSend }: Props) {
  const words = en.comments;
  const [name, setName] = useState("");
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (busy) {
      return;
    }
    setBusy(true);
    const sent = await onSend(name.trim(), text.trim());
    setBusy(false);
    if (sent) {
      setText("");
    }
  };

  return (
    <details>
      <summary>{words.reply}</summary>
      <div>
        <p className="muted">{words.replyHint}</p>
        <form onSubmit={submit}>
          <div className="field">
            <label htmlFor={`reply-name-${id}`}>{words.name}</label>
            <span className="hint" id={`reply-name-hint-${id}`}>
              {words.nameHint}
            </span>
            <input
              id={`reply-name-${id}`}
              value={name}
              required
              autoComplete="name"
              aria-describedby={`reply-name-hint-${id}`}
              onChange={(event) => setName(event.target.value)}
            />
          </div>
          <div className="field">
            <label htmlFor={`reply-text-${id}`}>{words.text}</label>
            <textarea
              id={`reply-text-${id}`}
              rows={4}
              value={text}
              required
              onChange={(event) => setText(event.target.value)}
            />
          </div>
          <button type="submit" className="btn" disabled={busy}>
            {words.send}
          </button>
        </form>
      </div>
    </details>
  );
}
