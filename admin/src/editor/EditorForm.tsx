import { useBlocker } from "@tanstack/react-router";
import { useState } from "react";
import type { AdminPost } from "../api/posts";
import { Problem } from "../components/Problem";
import { en } from "../strings/en";
import { GoLive } from "./GoLive";
import { LintPanel } from "./LintPanel";
import { PostSettings } from "./PostSettings";
import { PreviewLink } from "./PreviewLink";
import { PublishButton } from "./PublishButton";
import { RichText } from "./RichText";
import { fitsRichText } from "./richText";
import { scheduledAt } from "./schedule";
import { type SettingsDraft, withSetting } from "./settings";
import { type SaveState, useAutosave } from "./useAutosave";
import { usePublish } from "./usePublish";

type Props = { post: AdminPost };

function statusText(state: SaveState, dirty: boolean): string {
  if (state.kind === "saving") {
    return en.editor.saving;
  }
  return dirty || state.kind === "failed" ? en.editor.unsaved : en.editor.saved;
}

/** The title and text of one post, saved as the writer goes. */
export function EditorForm({ post }: Props) {
  const { draft, setDraft, dirty, state, save, updatedAt } = useAutosave(post);
  const { publishing, publish } = usePublish(post, save, updatedAt);
  const [rich, setRich] = useState(() => fitsRichText(post.markdown));
  const [held, setHeld] = useState(!rich);
  const [at, setAt] = useState(() => scheduledAt(post));
  const toRich = () => {
    const fits = fitsRichText(draft.markdown);
    setRich(fits);
    setHeld(!fits);
  };
  const leaving = useBlocker({
    shouldBlockFn: () => dirty,
    enableBeforeUnload: dirty,
    withResolver: true,
  });
  const onBlur = () => void save();
  const setTitle = (title: string) => setDraft((current) => ({ ...current, title }));
  const setMarkdown = (markdown: string) => setDraft((current) => ({ ...current, markdown }));
  const setSetting = <K extends keyof SettingsDraft>(key: K, value: SettingsDraft[K]) =>
    setDraft((current) => ({ ...current, settings: withSetting(current.settings, key, value) }));

  return (
    <>
      <h1 className="sr-only">{`${en.editor.heading} ${draft.title}`}</h1>
      <div className="bar">
        <fieldset className="switch" aria-label={en.editor.modes}>
          <button type="button" aria-pressed={rich} onClick={toRich}>
            {en.editor.rich}
          </button>
          <button type="button" aria-pressed={!rich} onClick={() => setRich(false)}>
            {en.editor.markdown}
          </button>
        </fieldset>
        <span className="hint" role="status">
          {statusText(state, dirty)}
        </span>
        <PublishButton
          publishing={publishing}
          scheduling={at !== ""}
          onPublish={() => void publish(at)}
        />
      </div>
      {publishing.problem === null ? null : <Problem message={publishing.problem} />}
      {leaving.status === "blocked" ? (
        <div className="problem" role="alertdialog" aria-labelledby="leave-what">
          <p id="leave-what">
            <strong>{en.editor.leave}</strong>
          </p>
          <div className="flex flex-wrap gap-3">
            <button type="button" className="btn primary" onClick={leaving.reset}>
              {en.editor.stay}
            </button>
            <button type="button" className="btn" onClick={leaving.proceed}>
              {en.editor.leaveAnyway}
            </button>
          </div>
        </div>
      ) : null}
      {held ? <p className="hint">{en.editor.onlyMarkdown}</p> : null}
      {state.kind === "failed" ? <Problem message={state.message} /> : null}
      <div className="layout">
        <div>
          <label className="sr-only" htmlFor="title">
            {en.editor.title}
          </label>
          <input
            id="title"
            className="title"
            value={draft.title}
            onChange={(event) => setTitle(event.target.value)}
            onBlur={onBlur}
          />
          {rich ? (
            <RichText markdown={draft.markdown} onChange={setMarkdown} onBlur={onBlur} />
          ) : (
            <textarea
              className="source"
              aria-label={en.editor.text}
              value={draft.markdown}
              onChange={(event) => setMarkdown(event.target.value)}
              onBlur={onBlur}
              rows={24}
              spellCheck
            />
          )}
        </div>
        <div>
          <LintPanel findings={publishing.findings} />
          <PostSettings settings={draft.settings} onSetting={setSetting} onBlur={onBlur}>
            {publishing.status === "published" ? null : <GoLive at={at} onAt={setAt} />}
            <PreviewLink id={post.id} />
          </PostSettings>
        </div>
      </div>
    </>
  );
}
