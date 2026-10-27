import { en } from "../strings/en";
import type { Publishing } from "./usePublish";

type Props = { publishing: Publishing; onPublish: () => void };

/** Publishes the post, and says why when the linter stopped it. */
export function PublishButton({ publishing, onPublish }: Props) {
  const blocked = publishing.findings.some((finding) => finding.severity === "block");
  return (
    <div className="flex flex-wrap items-center gap-3">
      {publishing.status === "published" ? (
        <span className="hint">{en.editor.published}</span>
      ) : null}
      <button
        type="button"
        className="btn primary"
        disabled={publishing.busy}
        aria-describedby={blocked ? "why-blocked" : undefined}
        onClick={onPublish}
      >
        {publishing.busy ? en.editor.publishing : en.editor.publish}
      </button>
      {blocked ? (
        <span id="why-blocked" className="hint">
          {en.editor.blocked}
        </span>
      ) : null}
    </div>
  );
}
