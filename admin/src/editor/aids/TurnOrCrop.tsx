import { useEffect, useRef, useState } from "react";
import { copyPicture, type MediaItem, usePicture } from "../../api/pictures";
import { pictures as words } from "../../strings/pictures";
import {
  cropFor,
  type Degrees,
  type PictureFacts,
  SHAPES,
  type Shape,
  turned,
} from "./pictureTurn";

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
  const [shape, setShape] = useState<Shape>("taken");
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => {
    form.current?.querySelector<HTMLElement>("button")?.focus();
  }, []);
  const facts = factsOf(picture);
  const save = async () => {
    const crop = facts === null ? null : cropFor(shape, degrees, facts);
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
        <img src={src} alt={words.preview} className={`turn-${degrees}`} />
      </div>
      <div className="flex flex-wrap gap-3 mb-4">
        <button type="button" className="btn" onClick={() => setDegrees(turned(degrees, false))}>
          {words.left}
        </button>
        <button type="button" className="btn" onClick={() => setDegrees(turned(degrees, true))}>
          {words.right}
        </button>
      </div>
      {facts === null ? null : (
        <fieldset className="mb-4">
          <legend>{words.shape}</legend>
          {SHAPES.map((each) => (
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
          <p className="hint">{words.shapeHint}</p>
        </fieldset>
      )}
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
