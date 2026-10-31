import { en } from "../strings/en";
import type { Finding, Score } from "./lint";

type Props = { findings: Finding[]; score: Score | null };

/** The linter's findings beside the post, each saying what, why and fix, under the score. */
export function LintPanel({ findings, score }: Props) {
  return (
    <aside className="checks" aria-labelledby="checks-title">
      <h2 id="checks-title">{en.editor.checks}</h2>
      {score === null ? null : (
        <div className="score">
          <b aria-hidden="true">{score.value}</b>
          <p>
            <strong>{en.editor.score(score.value)}</strong> {en.editor.scoreWhy(score.lost)}
          </p>
        </div>
      )}
      {findings.length === 0 ? <p className="hint">{en.editor.noChecks}</p> : null}
      {findings.map((finding) => {
        const block = finding.severity === "block";
        return (
          <section
            key={`${finding.rule} ${finding.subject ?? ""} ${finding.what}`}
            className={block ? "check block" : "check warn"}
            aria-label={block ? en.editor.blockTag : en.editor.warnTag}
          >
            <p className="tag">{block ? en.editor.blockTag : en.editor.warnTag}</p>
            <p>
              <strong>{finding.what}</strong>
              {finding.subject === null ? null : ` ${en.editor.here} "${finding.subject}".`}
            </p>
            <p>{finding.why}</p>
            <p>{finding.fix}</p>
          </section>
        );
      })}
    </aside>
  );
}
