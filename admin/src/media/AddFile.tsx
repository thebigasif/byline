import { useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { uploadMedia } from "../api/media";
import { problemMessage } from "../api/posts";
import { isData } from "../api/redirects";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";

/** The screen's one focus: a file into the library, with what became of it. */
export function AddFile() {
  const words = en.media.add;
  const queries = useQueryClient();
  const [file, setFile] = useState<File | null>(null);
  const [folder, setFolder] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const upload = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (busy) {
      return;
    }
    if (file === null) {
      setDone(null);
      setFailed(en.problems.noMedia);
      return;
    }
    setBusy(true);
    const reply = await uploadMedia(file, folder.trim());
    setBusy(false);
    if (!isData(reply)) {
      setDone(null);
      setFailed(problemMessage(reply.problem, en.problems.unreachable));
      return;
    }
    const { item, duplicate } = reply.data;
    const needsAlt = item.kind === "image" && item.alt === "";
    setFailed(null);
    if (duplicate) {
      setDone(words.duplicate(item.name));
    } else {
      setDone(needsAlt ? `${words.added(item.name)} ${words.addAlt}` : words.added(item.name));
    }
    await queries.invalidateQueries({ queryKey: ["media"] });
  };

  return (
    <>
      <section className="focus" aria-labelledby="add-title">
        <h2 id="add-title">{words.title}</h2>
        <form onSubmit={upload}>
          <div className="field">
            <label htmlFor="media-file">{words.file}</label>
            <span className="hint" id="media-file-hint">
              {words.fileHint}
            </span>
            <input
              id="media-file"
              type="file"
              aria-describedby="media-file-hint"
              onChange={(event) => setFile(event.target.files?.[0] ?? null)}
            />
          </div>
          <div className="field">
            <label htmlFor="media-folder">{words.folder}</label>
            <span className="hint" id="media-folder-hint">
              {words.folderHint}
            </span>
            <input
              id="media-folder"
              aria-describedby="media-folder-hint"
              value={folder}
              onChange={(event) => setFolder(event.target.value)}
            />
          </div>
          <button type="submit" className="btn primary">
            {words.submit}
          </button>
        </form>
      </section>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{busy ? words.uploading : done}</p>
    </>
  );
}
