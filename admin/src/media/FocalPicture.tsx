import type { MouseEvent } from "react";
import type { MediaItem } from "../api/media";
import { en } from "../strings/en";
import { type Percent, percent } from "./focal";

type Props = { item: MediaItem; point: Percent; onChange: (point: Percent) => void };

/** The image with a ring on its focal point; a click moves the ring there. */
export function FocalPicture({ item, point, onChange }: Props) {
  const pick = (event: MouseEvent<HTMLButtonElement>) => {
    // A key press also clicks a button; the two number fields serve the keyboard instead.
    if (event.detail === 0) {
      return;
    }
    const box = event.currentTarget.getBoundingClientRect();
    if (box.width === 0 || box.height === 0) {
      return;
    }
    onChange({
      x: percent(((event.clientX - box.left) / box.width) * 100),
      y: percent(((event.clientY - box.top) / box.height) * 100),
    });
  };
  return (
    <div>
      <button
        type="button"
        className="picture"
        aria-label={en.media.details.picture}
        aria-describedby="point-hint"
        onClick={pick}
      >
        <img
          src={item.url}
          alt=""
          width={item.width ?? undefined}
          height={item.height ?? undefined}
        />
        <svg className="ring" aria-hidden="true">
          <circle cx={`${point.x}%`} cy={`${point.y}%`} r="14" />
        </svg>
      </button>
      <p className="muted" id="point-hint">
        {en.media.details.pointHint}
      </p>
    </div>
  );
}
