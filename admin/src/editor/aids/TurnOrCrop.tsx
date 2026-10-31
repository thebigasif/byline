import { useEffect, useRef, useState } from "react";
import { copyPicture, type MediaItem, usePicture } from "../../api/pictures";
import { pictures as words } from "../../strings/pictures";
import { CropFields } from "./CropFields";
import { CropStage } from "./CropStage";
import { START } from "./cropBox";
import {
  type CropBox,
  cropFor,
  type Degrees,
  type PictureFacts,
  SHAPES,
  type Shape,
  turned,
} from "./pictureTurn";

/** A shape around the focal point, or the box the writer draws (decision E13). */
type Choice = Shape | "own";

/** Every choice, in the order the form offers them. */
const CHOICES: readonly Choice[] = [...SHAPES, "own"];

type Props = {
  /** The picture's library key. */
  id: string;
  /** Its address in the post, shown as the preview. */
  src: string;
  /** Hands back the copy's address, to put in the post. */
  onDone: (src: string) => void;
  onCancel: () => void;
};

/** The picture's size and focal point, once the blog has said; shapes need both. */
function factsOf(picture: MediaItem | null | undefined): PictureFacts | null {
  if (!picture?.width || !picture.height) {
    return null;
  }
  return { width: picture.width, height: picture.height, ...picture.focal_point };
}

/**
 * Turns a library picture in quarter turns and crops it to a shape around
 * its focal point, then saves the result as a new copy. It says before the
 * writer saves that the original stays.
 */
export function TurnOrCrop({ id, src, onDone, onCancel }: Props) {
  const picture = usePicture(id);
  const [degrees, setDegrees] = useState<Degrees>(0);
  const [shape, setShape] = useState<Choice>("taken");
  const [box, setBox] = useState<CropBox>(START);
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => {
    form.current?.querySelector<HTMLElement>("button")?.focus();
  }, []);
  const facts = factsOf(picture);
  // The sides swap on a turn, so a drawn box starts again.
  const turn = (right: boolean) => {
    setDegrees(turned(degrees, right));
    setBox(START);
  };
  const own = shape === "own" && facts !== null;
  const save = async () => {
    let crop: CropBox | null = null;
    if (own) {
      crop = box;
    } else if (facts !== null && shape !== "own") {
      crop = cropFor(shape, degrees, facts);
    }
    setSaving(true);
    const made = await copyPicture(id, { turn: degrees, crop }, words.copyFailed);
    setSaving(false);
    if ("url" in made) {
      onDone(made.url);
    } else {
      setFailed(made.failed);
    }
  };
  return (
    <form
      ref={form}
      className="focus block-form"
      aria-labelledby="turn-title"
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <h2 id="turn-title">{words.turnTitle}</h2>
      <p className="hint">{words.turnHint}</p>
      <div className="turn-preview">
        {own ? (
          <CropStage src={src} facts={facts} degrees={degrees} box={box} onChange={setBox} />
        ) : (
          <img src={src} alt={words.preview} className={`turn-${degrees}`} />
        )}
      </div>
      <div className="flex flex-wrap gap-3 mb-4">
        <button type="button" className="btn" onClick={() => turn(false)}>
          {words.left}
        </button>
        <button type="button" className="btn" onClick={() => turn(true)}>
          {words.right}
        </button>
      </div>
      {facts === null ? null : (
        <fieldset className="mb-4">
          <legend>{words.shape}</legend>
          {CHOICES.map((each) => (
            <label key={each} className="flex items-center gap-2">
              <input
                type="radio"
                name="shape"
                checked={shape === each}
                onChange={() => setShape(each)}
              />
              <span>{words.shapes[each]}</span>
            </label>
          ))}
          <p className="hint">{own ? words.ownHint : words.shapeHint}</p>
        </fieldset>
      )}
      {own ? <CropFields box={box} onChange={setBox} /> : null}
      {failed === null ? null : <p role="alert">{failed}</p>}
      <div className="flex flex-wrap gap-3">
        <button type="submit" className="btn primary" disabled={saving}>
          {saving ? words.saving : words.saveCopy}
        </button>
        <button type="button" className="btn" onClick={onCancel}>
          {words.cancel}
        </button>
      </div>
    </form>
  );
}
