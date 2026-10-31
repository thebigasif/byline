import type { PostPresence } from "../api/presence";
import { presence as words } from "../strings/presence";

type Props = {
  presence: PostPresence | null;
  onTakeOver: () => void;
};

/** Who else has the post open, or, when someone else holds it, why it is locked. */
export function Presence({ presence, onTakeOver }: Props) {
  if (presence === null) {
    return null;
  }
  if (presence.yours) {
    const names = presence.others.map((other) => other.who);
    return names.length === 0 ? null : <p className="hint">{words.others(names)}</p>;
  }
  const { who } = presence.holder;
  return (
    <section className="focus" aria-labelledby="held-title">
      <h2 id="held-title">{words.held(who)}</h2>
      <p>{words.why(who)}</p>
      <p className="hint">{words.cost(who)}</p>
      <button type="button" className="btn" onClick={onTakeOver}>
        {words.takeOver}
      </button>
    </section>
  );
}
