import { type RefObject, useState } from "react";
import { type AdminPost, problemMessage, savePost } from "../api/posts";
import { en, type UserMessage } from "../strings/en";
import { type Finding, lintFindings } from "./lint";

/** Where publishing stands: the post's status, and what stopped it, if anything. */
export type Publishing = {
  status: AdminPost["status"];
  busy: boolean;
  findings: Finding[];
  problem: UserMessage | null;
};

/**
 * Publishes the post once the draft is saved. A refusal from the publish
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

  const publish = async () => {
    setPublishing((current) => ({ ...current, busy: true }));
    if (!(await save())) {
      setPublishing((current) => ({ ...current, busy: false }));
      return;
    }
    const edit = { updated_at: updatedAt.current, status: "published" };
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
