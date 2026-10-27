import { useCallback, useEffect, useRef, useState } from "react";
import { type AdminPost, problemMessage, savePost } from "../api/posts";
import { en, type UserMessage } from "../strings/en";
import { type SettingsDraft, sameSettings, settingsDraft, settingsEdit } from "./settings";

/** What a writer changes in the editor: the words and the settings beside them. */
export type Draft = { title: string; markdown: string; settings: SettingsDraft };

/** Where saving stands, as the status line says it. */
export type SaveState =
  | { kind: "saved" }
  | { kind: "saving" }
  | { kind: "failed"; message: UserMessage };

/** How long the editor waits after the last keystroke before it saves. */
export const AUTOSAVE_MS = 3000;

const UNREACHABLE = { problem: null, status: 0 } as const;

function same(a: Draft, b: Draft): boolean {
  return a.title === b.title && a.markdown === b.markdown && sameSettings(a.settings, b.settings);
}

/**
 * Keeps a draft of `post` and saves it a few seconds after each change and
 * at once on `save`, sending the `updated_at` of the copy it was made on. A
 * 409 stops autosaving, so a change made elsewhere is never overwritten.
 */
export function useAutosave(post: AdminPost) {
  const [draft, setDraft] = useState<Draft>(() => ({
    title: post.title,
    markdown: post.markdown,
    settings: settingsDraft(post),
  }));
  const [state, setState] = useState<SaveState>({ kind: "saved" });
  const saved = useRef<Draft>(draft);
  const latest = useRef<Draft>(draft);
  const updatedAt = useRef(post.updated_at);
  const inFlight = useRef(false);
  const stopped = useRef(false);
  latest.current = draft;

  // True once the server holds the draft as it stands.
  const save = useCallback(async (): Promise<boolean> => {
    const sending = latest.current;
    if (inFlight.current || stopped.current) {
      return false;
    }
    if (same(sending, saved.current)) {
      return true;
    }
    inFlight.current = true;
    setState({ kind: "saving" });
    // Settings go only when changed, so an edit never touches one the writer left alone.
    const edit = {
      ...settingsEdit(saved.current.settings, sending.settings),
      updated_at: updatedAt.current,
      title: sending.title,
      markdown: sending.markdown,
    };
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
      return true;
    }
    stopped.current = answer.status === 409;
    setState({ kind: "failed", message: problemMessage(answer.problem, en.problems.notSaved) });
    return false;
  }, [post.id]);

  useEffect(() => {
    if (same(draft, saved.current)) {
      return;
    }
    const timer = setTimeout(() => void save(), AUTOSAVE_MS);
    return () => clearTimeout(timer);
  }, [draft, save]);

  const dirty = !same(draft, saved.current);
  return { draft, setDraft, dirty, state, save, updatedAt };
}
