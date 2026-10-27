import { type RefObject, useState } from "react";
import { type AdminPost, problemMessage, savePost } from "../api/posts";
import { en, type UserMessage } from "../strings/en";
import { type Finding, lintFindings } from "./lint";
import { goLiveEdit } from "./schedule";

/** Where publishing stands: the post's status, and what stopped it, if anything. */
export type Publishing = {
  status: AdminPost["status"];
  busy: boolean;
  findings: Finding[];
  problem: UserMessage | null;
};

/**
 * Publishes or schedules the post once the draft is saved. A refusal from the publish
 * gate brings back every lint finding, so the writer sees what to fix.
 */
export function usePublish(
  post: AdminPost,
  save: () => Promise<boolean>,
  updatedAt: RefObject<string>,
) {
  const [publishing, setPublishing] = useState<Publishing>({
    status: post.status,
    busy: false,
    findings: [],
    problem: null,
  });

  // `at` is the picked local time, or empty to publish now.
  const publish = async (at: string) => {
    if (goLiveEdit("", at) === null) {
      setPublishing((current) => ({ ...current, findings: [], problem: en.problems.badTime }));
      return;
    }
    setPublishing((current) => ({ ...current, busy: true }));
    const edit = (await save()) ? goLiveEdit(updatedAt.current, at) : null;
    if (edit === null) {
      setPublishing((current) => ({ ...current, busy: false }));
      return;
    }
    const answer = await savePost(post.id, edit).catch(() => null);
    if (answer !== null && "post" in answer) {
      updatedAt.current = answer.post.updated_at;
      setPublishing({ status: answer.post.status, busy: false, findings: [], problem: null });
      return;
    }
    const refusal = answer !== null && "problem" in answer ? answer.problem : null;
    const findings = lintFindings(refusal);
    const problem = findings.length > 0 ? null : problemMessage(refusal, en.problems.unreachable);
    setPublishing((current) => ({ ...current, busy: false, findings, problem }));
  };

  return { publishing, publish };
}
