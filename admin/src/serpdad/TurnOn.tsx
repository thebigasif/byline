import { en } from "../strings/en";

type Props = { busy: boolean; onMake: () => void };

/** The first thing an owner sees: what turning SerpDad on does, and the one button for it. */
export function TurnOn({ busy, onMake }: Props) {
  const words = en.serpdad.off;
  return (
    <section className="focus" aria-labelledby="serpdad-off-title">
      <h2 id="serpdad-off-title">{words.title}</h2>
      <p>{words.what}</p>
      <p>{words.after}</p>
      <button type="button" className="btn primary" disabled={busy} onClick={onMake}>
        {words.make}
      </button>
    </section>
  );
}
