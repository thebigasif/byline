import { en } from "../../strings/en";

/** One line of a writing-aid menu. */
export type MenuLine = { id: string; name: string; hint: string };

type Props = {
  label: string;
  lines: MenuLine[];
  active: number;
  /** What to say when nothing matches what the writer typed. */
  empty: string;
  onPick: (index: number) => void;
};

/**
 * A list the writer moves through with the arrow keys while the cursor stays
 * in the text. A click picks a line too, without taking the cursor away.
 */
export function AidMenu({ label, lines, active, empty, onPick }: Props) {
  return (
    <section className="aid-menu" aria-label={label}>
      <p className="aid-title">{label}</p>
      {lines.length === 0 ? (
        <p className="aid-help">{empty}</p>
      ) : (
        <ul>
          {lines.map((line, index) => (
            <li key={line.id}>
              <button
                type="button"
                tabIndex={-1}
                aria-current={index === active ? "true" : undefined}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => onPick(index)}
              >
                <b>{line.name}</b>
                <span>{line.hint}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      <p className="aid-help">{en.aids.help}</p>
    </section>
  );
}
