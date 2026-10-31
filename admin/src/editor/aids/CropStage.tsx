import { type PointerEvent, useRef } from "react";
import { pictures as words } from "../../strings/pictures";
import { type Drag, dragged, pointAt } from "./cropBox";
import {
  afterTurn,
  type CropBox,
  type Degrees,
  type PictureFacts,
  turnTransform,
} from "./pictureTurn";

type Props = {
  /** The picture's address. */
  src: string;
  /** Its size before turning. */
  facts: PictureFacts;
  degrees: Degrees;
  /** The part to keep, in thousandths of the turned picture. */
  box: CropBox;
  onChange: (box: CropBox) => void;
};

/**
 * The picture as it will be turned, with the part to keep in a box. Dragging
 * the box moves it; dragging the round handle at its corner changes its size
 * (decision E13). The fields beside it do the same without dragging.
 */
export function CropStage({ src, facts, degrees, box, onChange }: Props) {
  const { width, height } = afterTurn(facts, degrees);
  const drag = useRef<Drag | null>(null);
  const stage = useRef<SVGSVGElement>(null);
  const pointOf = (event: PointerEvent) => {
    const drawn = stage.current?.getBoundingClientRect();
    return drawn === undefined ? null : pointAt(event.clientX, event.clientY, drawn);
  };
  // One handler on the picture: the shape under the pointer says what moves.
  const start = (event: PointerEvent) => {
    const part = event.target instanceof Element ? event.target.getAttribute("data-part") : null;
    const from = pointOf(event);
    if ((part !== "box" && part !== "corner") || from === null) {
      return;
    }
    event.preventDefault();
    stage.current?.setPointerCapture?.(event.pointerId);
    drag.current = { part, from, box };
  };
  const follow = (event: PointerEvent) => {
    const to = drag.current === null ? null : pointOf(event);
    if (drag.current !== null && to !== null) {
      onChange(dragged(drag.current, to));
    }
  };
  const stop = () => {
    drag.current = null;
  };
  const x = (box.left * width) / 1000;
  const y = (box.top * height) / 1000;
  const w = (box.width * width) / 1000;
  const h = (box.height * height) / 1000;
  const handle = Math.max(width, height) / 40;
  return (
    <svg
      ref={stage}
      className="crop-stage"
      viewBox={`0 0 ${width} ${height}`}
      role="img"
      aria-label={words.stage}
      onPointerDown={start}
      onPointerMove={follow}
      onPointerUp={stop}
      onPointerCancel={stop}
    >
      <title>{words.stage}</title>
      <image
        href={src}
        width={facts.width}
        height={facts.height}
        transform={turnTransform(degrees, facts.width, facts.height)}
      />
      <path
        className="crop-shade"
        fillRule="evenodd"
        d={`M0 0H${width}V${height}H0Z M${x} ${y}h${w}v${h}h${-w}Z`}
      />
      <rect className="crop-box" data-part="box" x={x} y={y} width={w} height={h} />
      <circle className="crop-handle" data-part="corner" cx={x + w} cy={y + h} r={handle} />
    </svg>
  );
}
