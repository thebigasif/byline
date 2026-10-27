import { useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { problemMessage } from "../api/posts";
import { exportRedirects, type Failure, importRedirects, isData } from "../api/redirects";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { KEPT } from "./KeptRedirects";

/** Saves `text` as a file the browser downloads, named `name`. */
function save(text: string, name: string) {
  const address = URL.createObjectURL(new Blob([text], { type: "text/csv" }));
  const link = document.createElement("a");
  link.href = address;
  link.download = name;
  link.click();
  URL.revokeObjectURL(address);
}

/** Every kept redirect out as a CSV file, and a CSV file of redirects in, whole or not at all. */
export function CsvRedirects() {
  const words = en.redirects.csv;
  const queries = useQueryClient();
  const [file, setFile] = useState<File | null>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const fail = ({ problem }: Failure) => {
    setDone(null);
    setFailed(problemMessage(problem, en.problems.unreachable));
  };

  const download = async () => {
    const reply = await exportRedirects();
    if (isData(reply)) {
      setFailed(null);
      save(reply.data, "redirects.csv");
    } else {
      fail(reply);
    }
  };

  const upload = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (busy) {
      return;
    }
    if (file === null) {
      setDone(null);
      setFailed(en.problems.noFile);
      return;
    }
    setBusy(true);
    const reply = await importRedirects(await file.text());
    setBusy(false);
    if (isData(reply)) {
      setFailed(null);
      setDone(words.added(reply.data.length));
      await queries.invalidateQueries({ queryKey: KEPT });
    } else {
      fail(reply);
    }
  };

  return (
    <section aria-labelledby="csv-title">
      <h2 id="csv-title">{words.title}</h2>
      <p className="muted">{words.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{busy ? words.adding : done}</p>
      <p>
        <button type="button" className="btn" onClick={download}>
          {words.download}
        </button>
      </p>
      <form onSubmit={upload}>
        <div className="field">
          <label htmlFor="csv">{words.file}</label>
          <span className="hint" id="csv-hint">
            {words.fileHint}
          </span>
          <input
            id="csv"
            type="file"
            accept=".csv,text/csv"
            aria-describedby="csv-hint"
            onChange={(event) => setFile(event.target.files?.[0] ?? null)}
          />
        </div>
        <button type="submit" className="btn">
          {words.add}
        </button>
      </form>
    </section>
  );
}
