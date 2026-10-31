import { en } from "../../strings/en";

const labels: Record<string, string> = en.fields;
const choiceNames: Record<string, string> = en.choices;

/** The details that take one of a few words, the default first. */
const CHOICES: Record<string, readonly string[]> = {
  kind: ["note", "tip", "warning"],
  show: ["everyone", "visitors", "members"],
};

type Props = {
  field: string;
  value: string;
  onChange: (value: string) => void;
};

/** One labelled detail of a block: a list of choices, the block's text, or a line. */
export function DetailField({ field, value, onChange }: Props) {
  const id = `block-${field}`;
  const choices = CHOICES[field];
  let input = <input id={id} value={value} onChange={(event) => onChange(event.target.value)} />;
  if (choices !== undefined) {
    input = (
      <select
        id={id}
        value={value === "" ? choices[0] : value}
        onChange={(event) => onChange(event.target.value)}
      >
        {choices.map((choice) => (
          <option key={choice} value={choice}>
            {choiceNames[choice] ?? choice}
          </option>
        ))}
      </select>
    );
  } else if (field === "body") {
    input = (
      <textarea
        id={id}
        className="source"
        value={value}
        rows={6}
        onChange={(event) => onChange(event.target.value)}
      />
    );
  }
  return (
    <div className="field">
      <label htmlFor={id}>{labels[field] ?? field}</label>
      {input}
    </div>
  );
}
