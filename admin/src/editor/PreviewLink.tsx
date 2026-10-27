import { useState } from "react";
import { type PreviewLink as Made, makePreviewLink, problemMessage } from "../api/posts";
import { Problem } from "../components/Problem";
import { day } from "../strings/dates";
import { en, type UserMessage } from "../strings/en";

type Props = { id: string };

// The clipboard may be missing or refused; the link stays on screen to copy by hand.
async function copy(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/** Makes a link that shows the draft to someone without an account, and copies it. */
export function PreviewLink({ id }: Props) {
  const [link, setLink] = useState<Made | null>(null);
  const [copied, setCopied] = useState(false);
  const [problem, setProblem] = useState<UserMessage | null>(null);
  const [busy, setBusy] = useState(false);

  const make = async () => {
    setBusy(true);
    const answer = await makePreviewLink(id).catch(() => null);
    setBusy(false);
    if (answer === null || !("link" in answer)) {
      setProblem(problemMessage(answer?.problem ?? null, en.problems.unreachable));
      return;
    }
    setProblem(null);
    setLink(answer.link);
    setCopied(await copy(answer.link.url));
  };

  return (
    <div className="field">
      <h3 id="preview-title">{en.preview.title}</h3>
      <p className="hint" id="preview-hint">
        {en.preview.hint}
      </p>
      <button
        type="button"
        className="btn"
        disabled={busy}
        aria-describedby="preview-title preview-hint"
        onClick={() => void make()}
      >
        {en.preview.make}
      </button>
      {problem === null ? null : <Problem message={problem} />}
      {link === null ? null : (
        <div className="mt-3">
          <label htmlFor="preview-url">{en.preview.label}</label>
          <span className="hint" id="preview-url-hint">
            {en.preview.until(copied, day(link.expires_at))}
          </span>
          <input id="preview-url" readOnly value={link.url} aria-describedby="preview-url-hint" />
        </div>
      )}
    </div>
  );
}
