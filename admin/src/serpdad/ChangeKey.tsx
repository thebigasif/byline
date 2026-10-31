import { en } from "../strings/en";

type Props = { busy: boolean; onRenew: () => void; onOff: () => void };

/** The quiet way out once connected: a new key in place of the old, or no key at all. */
export function ChangeKey({ busy, onRenew, onOff }: Props) {
  const words = en.serpdad.change;
  return (
    <section aria-labelledby="serpdad-change-title">
      <h2 id="serpdad-change-title">{words.title}</h2>
      <p className="muted">{words.lede}</p>
      <div className="flex flex-wrap gap-3">
        <button type="button" className="btn" disabled={busy} onClick={onRenew}>
          {words.renew}
        </button>
        <button type="button" className="btn" disabled={busy} onClick={onOff}>
          {words.off}
        </button>
      </div>
    </section>
  );
}
