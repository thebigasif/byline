type Option = { value: string; label: string };

type Props = {
  id: string;
  label: string;
  /** The first option, which filters nothing. */
  any: string;
  value: string;
  options: Option[];
  onChange: (value: string) => void;
};

/** One labelled filter: "any" first, then each value the list can be narrowed to. */
export function SelectFilter({ id, label, any, value, options, onChange }: Props) {
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <select id={id} name={id} value={value} onChange={(event) => onChange(event.target.value)}>
        <option value="">{any}</option>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}
