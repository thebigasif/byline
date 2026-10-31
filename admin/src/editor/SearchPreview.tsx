import type { ChangeEvent } from "react";
import type { Snippet } from "../api/check";
import { en } from "../strings/en";
import { SettingField } from "./SettingField";
import type { SettingsDraft, TextSetting } from "./settings";
import { shownAddress } from "./snippet";

type Props = {
  /** The result the saved post makes, or null before the blog has checked it. */
  snippet: Snippet | null;
  settings: SettingsDraft;
  onSetting: (key: TextSetting, value: string) => void;
  onBlur: () => void;
};

/** The post as a Google result, with the title and keyword that shape it. */
export function SearchPreview({ snippet, settings, onSetting, onBlur }: Props) {
  const words = en.search;
  const bind = (key: "seoTitle" | "focusKeyword") => ({
    id: `setting-${key}`,
    value: settings[key],
    "aria-describedby": `setting-${key}-hint`,
    onChange: (event: ChangeEvent<HTMLInputElement>) => onSetting(key, event.target.value),
    onBlur,
  });
  const lengths = snippet && words.lengths(snippet.title.length, snippet.description.length);
  return (
    <details open>
      <summary>{words.title}</summary>
      <div>
        <p className="hint">{words.lede}</p>
        {snippet === null ? null : (
          <>
            <section className="serp" aria-label={words.result}>
              <p className="url">{shownAddress(snippet.url)}</p>
              <p className="head">{snippet.title}</p>
              <p className="desc">{snippet.description || words.none}</p>
            </section>
            <p className="hint">{lengths}</p>
          </>
        )}
        <SettingField id="setting-seoTitle" label={words.seoTitle} hint={words.seoTitleHint}>
          <input {...bind("seoTitle")} />
        </SettingField>
        <p className="hint">{words.description}</p>
        <SettingField id="setting-focusKeyword" label={words.focus} hint={words.focusHint}>
          <input {...bind("focusKeyword")} />
        </SettingField>
      </div>
    </details>
  );
}
