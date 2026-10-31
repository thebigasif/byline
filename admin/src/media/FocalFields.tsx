import { en } from "../strings/en";
import { type Percent, percent } from "./focal";

type Props = { point: Percent; onChange: (point: Percent) => void };

/** The focal point as two numbers, for the keyboard and screen readers. */
export function FocalFields({ point, onChange }: Props) {
  const words = en.media.details;
  return (
    <fieldset>
      <legend>{words.point}</legend>
      <div className="pair">
        <div className="field">
          <label htmlFor="point-x">{words.left}</label>
          <input
            id="point-x"
            type="number"
            min={0}
            max={100}
            value={point.x}
            onChange={(event) => onChange({ ...point, x: percent(event.target.valueAsNumber) })}
          />
        </div>
        <div className="field">
          <label htmlFor="point-y">{words.top}</label>
          <input
            id="point-y"
            type="number"
            min={0}
            max={100}
            value={point.y}
            onChange={(event) => onChange({ ...point, y: percent(event.target.valueAsNumber) })}
          />
        </div>
      </div>
    </fieldset>
  );
}
