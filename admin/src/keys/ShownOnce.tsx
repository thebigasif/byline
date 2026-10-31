import type { AdminKey } from "../api/keys";
import { en } from "../strings/en";

type Props = { made: AdminKey };

/** A new key, shown this once in a field to copy from; it is never shown again. */
export function ShownOnce({ made }: Props) {
  const words = en.keys.shown;
  return (
    <div className="card" role="status">
      <p>
        <strong>{words.title(made.name)}</strong>
      </p>
      <div className="field">
        <label htmlFor="made-key">{words.label}</label>
        <span className="hint" id="made-key-hint">
          {words.hint}
        </span>
        <input
          id="made-key"
          readOnly
          spellCheck={false}
          aria-describedby="made-key-hint"
          value={made.key ?? ""}
          onFocus={(event) => event.target.select()}
        />
      </div>
    </div>
  );
}
