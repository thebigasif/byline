import { useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { editMedia, type MediaEdit, type MediaItem } from "../api/media";
import { problemMessage } from "../api/posts";
import { isData } from "../api/redirects";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";

type Props = { item: MediaItem };

type Draft = { alt: string; caption: string; folder: string };

/** The fields that changed from `item`, on the copy its `updated_at` names. */
function changes(item: MediaItem, draft: Draft): MediaEdit {
  const edit: MediaEdit = { updated_at: item.updated_at };
  if (draft.alt !== item.alt) {
    edit.alt = draft.alt;
  }
  if (draft.caption !== item.caption) {
    edit.caption = draft.caption;
  }
  if (draft.folder.trim() !== item.folder) {
    edit.folder = draft.folder.trim();
  }
  return edit;
}

/** One file's alt text, caption and folder, saved together. */
export function DetailsForm({ item }: Props) {
  const words = en.media.details;
  const queries = useQueryClient();
  const image = item.kind === "image";
  const [draft, setDraft] = useState<Draft>({
    alt: item.alt,
    caption: item.caption,
    folder: item.folder,
  });
  // The copy the next edit is made on; it moves on with each save.
  const [saved, setSaved] = useState(item);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);
  const change = (fields: Partial<Draft>) => setDraft({ ...draft, ...fields });

  const save = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (busy) {
      return;
    }
    setBusy(true);
    const reply = await editMedia(item.id, changes(saved, draft));
    setBusy(false);
    if (!isData(reply)) {
      setDone(null);
      setFailed(problemMessage(reply.problem, en.problems.unreachable));
      return;
    }
    setFailed(null);
    setDone(words.saved);
    setSaved(reply.data);
    queries.setQueryData(["media-item", item.id], reply);
    await queries.invalidateQueries({ queryKey: ["media"] });
  };

  return (
    <section className="focus" aria-labelledby="details-title">
      <h2 id="details-title">{words.title}</h2>
      {failed === null ? null : <Problem message={failed} />}
      <form onSubmit={save}>
        {image ? (
          <div className="field">
            <label htmlFor="media-alt">{words.alt}</label>
            <span className="hint" id="media-alt-hint">
              {words.altHint}
            </span>
            <textarea
              id="media-alt"
              rows={3}
              aria-describedby="media-alt-hint"
              value={draft.alt}
              onChange={(event) => change({ alt: event.target.value })}
            />
          </div>
        ) : null}
        <div className="field">
          <label htmlFor="media-caption">{words.caption}</label>
          <span className="hint" id="media-caption-hint">
            {words.captionHint}
          </span>
          <input
            id="media-caption"
            aria-describedby="media-caption-hint"
            value={draft.caption}
            onChange={(event) => change({ caption: event.target.value })}
          />
        </div>
        <div className="field">
          <label htmlFor="media-folder">{en.media.library.folder}</label>
          <span className="hint" id="media-folder-hint">
            {words.folderHint}
          </span>
          <input
            id="media-folder"
            aria-describedby="media-folder-hint"
            value={draft.folder}
            onChange={(event) => change({ folder: event.target.value })}
          />
        </div>
        <button type="submit" className="btn primary">
          {words.save}
        </button>
      </form>
      <p role="status">{busy ? words.saving : done}</p>
    </section>
  );
}
