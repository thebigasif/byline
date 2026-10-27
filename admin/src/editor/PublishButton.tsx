import { en } from "../strings/en";
import type { Publishing } from "./usePublish";

type Props = { publishing: Publishing; scheduling: boolean; onPublish: () => void };

function label(busy: boolean, scheduling: boolean): string {
  if (scheduling) {
    return busy ? en.editor.scheduling : en.editor.schedule;
  }
  return busy ? en.editor.publishing : en.editor.publish;
}

/** Publishes the post, or schedules it for a picked time, and says why the linter stopped it. */
export function PublishButton({ publishing, scheduling, onPublish }: Props) {
  const blocked = publishing.findings.some((finding) => finding.severity === "block");
  const where = { draft: null, published: en.editor.published, scheduled: en.editor.scheduled };
  const status = where[publishing.status];
  return (
    <div className="flex flex-wrap items-center gap-3">
      {status === null ? null : <span className="hint">{status}</span>}
      <button
        type="button"
        className="btn primary"
        disabled={publishing.busy}
        aria-describedby={blocked ? "why-blocked" : undefined}
        onClick={onPublish}
      >
        {label(publishing.busy, scheduling)}
      </button>
      {blocked ? (
        <span id="why-blocked" className="hint">
          {en.editor.blocked}
        </span>
      ) : null}
    </div>
  );
}
