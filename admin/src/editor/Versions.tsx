import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { problemMessage } from "../api/posts";
import { listVersions, type PostVersion } from "../api/versions";
import { Problem } from "../components/Problem";
import { moment } from "../strings/dates";
import { en } from "../strings/en";
import { lineDiff } from "./diff";

type Words = { title: string; markdown: string };

type Props = {
  id: string;
  /** When the post last saved; the list loads again each time it changes. */
  version: string;
  /** The title and text in the editor now. */
  now: Words;
  /** Puts an earlier title and text in the editor. */
  onRestore: (version: PostVersion) => void;
};

/** The title and the text as one comparable text, the title first. */
const whole = (words: Words) => `${words.title}\n\n${words.markdown}`;

/** The lines that changed from `then` to `now`, marked removed or added. */
function Changes({ then, now }: { then: Words; now: Words }) {
  const lines = lineDiff(whole(then), whole(now)).map((line, n) => ({ ...line, id: `line-${n}` }));
  return (
    <div className="diff" role="group" aria-label={en.versions.changes}>
      {lines.map((line) => {
        if (line.kind === "removed") {
          return (
            <del key={line.id} title={en.versions.removed}>
              {line.text}
            </del>
          );
        }
        if (line.kind === "added") {
          return (
            <ins key={line.id} title={en.versions.added}>
              {line.text}
            </ins>
          );
        }
        return <p key={line.id}>{line.text}</p>;
      })}
    </div>
  );
}

type PickingProps = Pick<Props, "now" | "onRestore"> & { versions: PostVersion[] };

/** The versions to pick from, then what changed since the one picked. */
function Picking({ versions, now, onRestore }: PickingProps) {
  const [chosen, setChosen] = useState<number | null>(null);
  const picked = chosen === null ? undefined : versions[chosen];
  const listed = versions.map((kept, n) => ({ kept, n, id: `version-${n}` }));
  return (
    <>
      <ol className="versions" aria-label={en.versions.list}>
        {listed.map(({ kept, n, id }) => (
          <li key={id}>
            <button type="button" aria-pressed={chosen === n} onClick={() => setChosen(n)}>
              {`${moment(kept.saved_at)}${en.versions.by(kept.saved_by)}`}
            </button>
          </li>
        ))}
      </ol>
      {picked === undefined ? null : (
        <>
          <Changes then={picked} now={now} />
          <button type="button" className="btn primary" onClick={() => onRestore(picked)}>
            {en.versions.restore}
          </button>
          <p className="hint">{en.versions.restoreHint}</p>
        </>
      )}
    </>
  );
}

/** A post's earlier versions: what changed since each, and a way to bring one back. */
export function Versions({ id, version, now, onRestore }: Props) {
  const [open, setOpen] = useState(false);
  const answer = useQuery({
    queryKey: ["versions", id, version],
    queryFn: () => listVersions(id),
    enabled: open,
  }).data;
  let body = null;
  if (answer !== undefined && "versions" in answer) {
    body =
      answer.versions.length === 0 ? (
        <p className="hint">{en.versions.none}</p>
      ) : (
        <Picking versions={answer.versions} now={now} onRestore={onRestore} />
      );
  } else if (answer !== undefined) {
    body = <Problem message={problemMessage(answer.problem, en.problems.unreachable)} />;
  }
  return (
    <div className="field">
      <h3>{en.versions.title}</h3>
      <p className="hint">{en.versions.hint}</p>
      {open ? (
        body
      ) : (
        <button type="button" className="btn" onClick={() => setOpen(true)}>
          {en.versions.show}
        </button>
      )}
    </div>
  );
}
