import { useState } from "react";
import { en } from "../strings/en";

type Props = { secret: string };

// The clipboard may be missing or refused; the key stays on screen to copy by hand.
async function copy(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/** The new key, shown this once, beside the blog's address SerpDad also needs. */
export function KeyShown({ secret }: Props) {
  const words = en.serpdad.key;
  const [copied, setCopied] = useState<boolean | null>(null);
  const copyKey = async () => setCopied(await copy(secret));
  let told: string | null = null;
  if (copied !== null) {
    told = copied ? words.copied : words.notCopied;
  }
  return (
    <section className="focus wide" aria-labelledby="serpdad-key-title">
      <h2 id="serpdad-key-title">{words.title}</h2>
      <p>{words.where}</p>
      <div className="field">
        <label htmlFor="serpdad-address">{words.address}</label>
        <input id="serpdad-address" readOnly value={globalThis.location.origin} />
      </div>
      <div className="field">
        <label htmlFor="serpdad-key">{words.key}</label>
        <input id="serpdad-key" readOnly value={secret} />
      </div>
      <p className="once">
        <strong>{words.once}</strong> {words.onceWhy}
      </p>
      <button type="button" className="btn primary" onClick={() => void copyKey()}>
        {words.copy}
      </button>
      <p role="status">{told}</p>
    </section>
  );
}
