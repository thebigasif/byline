import { useEffect, useRef, useState } from "react";
import { en } from "../../strings/en";
import { pictures } from "../../strings/pictures";
import { type Details, fieldsOf } from "./blockForm";
import { DetailField } from "./DetailField";
import { libraryKey } from "./pictureTurn";
import { TurnOrCrop } from "./TurnOrCrop";

const names: Record<string, string> = en.blocks;
const hints: Record<string, string> = en.blockHints;

type Props = {
  details: Details;
  onSave: (values: Record<string, string>) => void;
  onCancel: () => void;
};

/** A block's details, asked for when it is added or when the writer opens it. */
export function BlockForm({ details, onSave, onCancel }: Props) {
  const { spec } = details;
  const [values, setValues] = useState(details.values);
  const [turning, setTurning] = useState(false);
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => {
    form.current?.querySelector<HTMLElement>("input, select, textarea")?.focus();
  }, []);
  // Only a picture the library keeps can be turned or cropped (decision E10).
  const key = spec.name === "figure" ? libraryKey(values.src ?? "") : null;
  if (turning && key !== null) {
    return (
      <TurnOrCrop
        id={key}
        src={values.src ?? ""}
        onDone={(src) => onSave({ ...values, src })}
        onCancel={() => setTurning(false)}
      />
    );
  }
  return (
    <form
      ref={form}
      className="focus block-form"
      aria-labelledby="block-form-title"
      onSubmit={(event) => {
        event.preventDefault();
        onSave(values);
      }}
    >
      <h2 id="block-form-title">{names[spec.name] ?? spec.name}</h2>
      <p className="hint">{hints[spec.name]}</p>
      {fieldsOf(spec).map((field) => (
        <DetailField
          key={field}
          field={field}
          value={values[field] ?? ""}
          onChange={(value) => setValues((current) => ({ ...current, [field]: value }))}
        />
      ))}
      <div className="flex flex-wrap gap-3">
        <button type="submit" className="btn primary">
          {details.editing ? en.details.save : en.details.add}
        </button>
        {key === null ? null : (
          <button type="button" className="btn" onClick={() => setTurning(true)}>
            {pictures.turnOpen}
          </button>
        )}
        <button type="button" className="btn" onClick={onCancel}>
          {en.details.cancel}
        </button>
      </div>
    </form>
  );
}
