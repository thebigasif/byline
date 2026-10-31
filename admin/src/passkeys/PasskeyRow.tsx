import { useState } from "react";
import type { Passkey } from "../api/passkeys";
import { day, moment } from "../strings/dates";
import { en } from "../strings/en";

type Props = {
  passkey: Passkey;
  onRemove: (passkey: Passkey) => void;
};

/** What the row's last cell shows: the buttons, or one more question. */
type Doing = "idle" | "asking";

/** One passkey, removed after one more question in its own row. */
export function PasskeyRow({ passkey, onRemove }: Props) {
  const words = en.passkeys;
  const [doing, setDoing] = useState<Doing>("idle");

  let action = (
    <div className="flex flex-wrap items-center gap-3">
      <button
        type="button"
        className="btn"
        aria-label={words.remove(passkey.name)}
        onClick={() => setDoing("asking")}
      >
        {words.removeText}
      </button>
    </div>
  );
  if (doing === "asking") {
    action = (
      <div className="flex flex-wrap items-center gap-3">
        <span>{words.asking(passkey.name)}</span>
        <button
          type="button"
          className="btn"
          aria-label={words.sure(passkey.name)}
          onClick={() => onRemove(passkey)}
        >
          {words.sureText}
        </button>
        <button type="button" className="btn" onClick={() => setDoing("idle")}>
          {words.keep}
        </button>
      </div>
    );
  }

  const used = passkey.last_used_at ?? null;
  return (
    <tr>
      <td>{passkey.name}</td>
      <td data-label={words.added}>
        <time dateTime={passkey.added_at}>{day(passkey.added_at)}</time>
      </td>
      <td data-label={words.lastUsed}>
        {used === null ? words.never : <time dateTime={used}>{moment(used)}</time>}
      </td>
      <td data-label={words.action}>{action}</td>
    </tr>
  );
}
