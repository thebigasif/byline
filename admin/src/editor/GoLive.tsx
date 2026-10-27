import { useState } from "react";
import { en } from "../strings/en";

type Props = { at: string; onAt: (at: string) => void };

/** When the post goes live: on Publish, or at a picked time in the writer's time zone. */
export function GoLive({ at, onAt }: Props) {
  const [picking, setPicking] = useState(at !== "");
  const now = () => {
    setPicking(false);
    onAt("");
  };
  return (
    <fieldset>
      <legend>{en.goLive.title}</legend>
      <span className="hint">{en.goLive.hint}</span>
      <label className="choice">
        <input type="radio" name="go-live" checked={!picking} onChange={now} />
        <span>{en.goLive.now}</span>
      </label>
      <label className="choice">
        <input type="radio" name="go-live" checked={picking} onChange={() => setPicking(true)} />
        <span>{en.goLive.later}</span>
      </label>
      {picking ? (
        <div className="field">
          <label htmlFor="go-live-at">{en.goLive.at}</label>
          <span className="hint" id="go-live-at-hint">
            {en.goLive.atHint}
          </span>
          <input
            id="go-live-at"
            type="datetime-local"
            aria-describedby="go-live-at-hint"
            value={at}
            onChange={(event) => onAt(event.target.value)}
          />
        </div>
      ) : null}
    </fieldset>
  );
}
