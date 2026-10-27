import { useCallback, useEffect, useRef, useState } from "react";
import { type AdminPost, problemMessage, savePost } from "../api/posts";
import { en, type UserMessage } from "../strings/en";

/** The words of a post a writer changes in the editor. */
export type Draft = { title: string; markdown: string };

/** Where saving stands, as the status line says it. */
export type SaveState =
  | { kind: "saved" }
  | { kind: "saving" }
  | { kind: "failed"; message: UserMessage };

/** How long the editor waits after the last keystroke before it saves. */
export const AUTOSAVE_MS = 3000;

const UNREACHABLE = { problem: null, status: 0 } as const;

function same(a: Draft, b: Draft): boolean {
  return a.title === b.title && a.markdown === b.markdown;
}

/**
 * Keeps a draft of `post` and saves it a few seconds after each change and
 * at once on `save`, sending the `updated_at` of the copy it was made on. A
 * 409 stops autosaving, so a change made elsewhere is never overwritten.
 */
export function useAutosave(post: AdminPost) {
  const [draft, setDraft] = useState<Draft>({ title: post.title, markdown: post.markdown });
  const [state, setState] = useState<SaveState>({ kind: "saved" });
  const saved = useRef<Draft>(draft);
  const latest = useRef<Draft>(draft);
  const updatedAt = useRef(post.updated_at);
  const inFlight = useRef(false);
  const stopped = useRef(false);
  latest.current = draft;

  const save = useCallback(async () => {
    const sending = latest.current;
    if (same(sending, saved.current) || inFlight.current || stopped.current) {
      return;
    }
    inFlight.current = true;
    setState({ kind: "saving" });
    const edit = { updated_at: updatedAt.current, ...sending };
    const answer = await savePost(post.id, edit).catch(() => UNREACHABLE);
    inFlight.current = false;
    if ("post" in answer) {
      saved.current = sending;
      updatedAt.current = answer.post.updated_at;
      setState({ kind: "saved" });
      if (!same(latest.current, sending)) {
        // Typed while saving: wait for a pause, then save that too.
        setDraft({ ...latest.current });
      }
      return;
    }
    stopped.current = answer.status === 409;
    setState({ kind: "failed", message: problemMessage(answer.problem, en.problems.notSaved) });
  }, [post.id]);

  useEffect(() => {
    if (same(draft, saved.current)) {
      return;
    }
    const timer = setTimeout(() => void save(), AUTOSAVE_MS);
    return () => clearTimeout(timer);
  }, [draft, save]);

  const dirty = !same(draft, saved.current);
  return { draft, setDraft, dirty, state, save };
}
