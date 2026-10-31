import { pictures as words } from "../../strings/pictures";
import { percentOf, SIDES, withPercent } from "./cropBox";
import type { CropBox } from "./pictureTurn";

type Props = {
  box: CropBox;
  onChange: (box: CropBox) => void;
};

/** The crop box in four fields, in whole percent, for writers who do not drag (WCAG 2.5.7). */
export function CropFields({ box, onChange }: Props) {
  return (
    <div className="crop-fields">
      {SIDES.map((side) => (
        <div key={side}>
          <label htmlFor={`crop-${side}`}>{words.sides[side]}</label>
          <input
            id={`crop-${side}`}
            type="number"
            inputMode="numeric"
            min={side === "left" || side === "top" ? 0 : 5}
            max={100}
            value={percentOf(box, side)}
            onChange={(event) => {
              onChange(withPercent(box, side, Number.parseFloat(event.target.value)));
            }}
          />
        </div>
      ))}
    </div>
  );
}
