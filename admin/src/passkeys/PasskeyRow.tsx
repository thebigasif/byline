import { type FormEvent, useState } from "react";
import type { Passkey } from "../api/passkeys";
import { day, moment } from "../strings/dates";
import { en } from "../strings/en";

type Props = {
  passkey: Passkey;
  onRename: (passkey: Passkey, name: string) => Promise<boolean>;
  onRemove: (passkey: Passkey) => void;
};

/** What the row's last cell shows: the two buttons, a new name, or one more question. */
type Doing = "idle" | "renaming" | "asking";

/** One passkey, renamed in its own row, and removed after one more question there. */
export function PasskeyRow({ passkey, onRename, onRemove }: Props) {
  const words = en.passkeys;
  const [doing, setDoing] = useState<Doing>("idle");
  const [name, setName] = useState(passkey.name);
  const field = `passkey-name-${passkey.id}`;

  const save = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (await onRename(passkey, name.trim())) {
      setDoing("idle");
    }
  };

  let action = (
    <div className="flex flex-wrap items-center gap-3">
      <button
        type="button"
        className="btn"
        aria-label={words.rename(passkey.name)}
        onClick={() => setDoing("renaming")}
      >
        {words.renameText}
      </button>
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
  if (doing === "renaming") {
    action = (
      <form className="flex flex-wrap items-end gap-3" onSubmit={save}>
        <div className="field">
          <label htmlFor={field}>{words.newName(passkey.name)}</label>
          <input
            id={field}
            value={name}
            maxLength={60}
            required
            onChange={(event) => setName(event.target.value)}
          />
        </div>
        <button type="submit" className="btn">
          {words.saveName}
        </button>
        <button type="button" className="btn" onClick={() => setDoing("idle")}>
          {words.keepName}
        </button>
      </form>
    );
  } else if (doing === "asking") {
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
