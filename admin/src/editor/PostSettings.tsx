import type { ChangeEvent, ReactNode } from "react";
import { en } from "../strings/en";
import { MoreSettings } from "./MoreSettings";
import { SettingField } from "./SettingField";
import { REVIEW_STATES, type SettingsDraft, type TextSetting } from "./settings";

type Props = {
  settings: SettingsDraft;
  onSetting: <K extends keyof SettingsDraft>(key: K, value: SettingsDraft[K]) => void;
  onBlur: () => void;
  /** Settings that are not saved with the text, shown after the rest. */
  children?: ReactNode;
};

type Typed = ChangeEvent<HTMLInputElement | HTMLTextAreaElement>;

/** The post's settings beside the text, each saved with it. */
export function PostSettings({ settings, onSetting, onBlur, children }: Props) {
  const text = (key: TextSetting) => ({
    id: `setting-${key}`,
    value: settings[key],
    "aria-describedby": `setting-${key}-hint`,
    onChange: (event: Typed) => onSetting(key, event.target.value),
    onBlur,
  });
  const review = (event: ChangeEvent<HTMLSelectElement>) => {
    const state = REVIEW_STATES.find((each) => each === event.target.value);
    if (state !== undefined) {
      onSetting("reviewState", state);
    }
  };
  const words = en.settings;

  return (
    <details className="settings" open>
      <summary>{words.title}</summary>
      <div>
        <SettingField id="setting-slug" label={words.slug} hint={words.slugHint}>
          <input {...text("slug")} spellCheck={false} />
        </SettingField>
        <SettingField id="setting-author" label={words.author} hint={words.authorHint}>
          <input {...text("author")} spellCheck={false} />
        </SettingField>
        <SettingField id="setting-coAuthors" label={words.coAuthors} hint={words.coAuthorsHint}>
          <input {...text("coAuthors")} spellCheck={false} />
        </SettingField>
        <SettingField id="setting-date" label={words.date} hint={words.dateHint}>
          <input {...text("date")} type="date" />
        </SettingField>
        <SettingField
          id="setting-excerpt"
          label={words.excerpt}
          hint={words.excerptHint(settings.excerpt.length)}
        >
          <textarea {...text("excerpt")} rows={4} />
        </SettingField>
        <SettingField id="setting-review" label={words.review} hint={words.reviewHint}>
          <select
            id="setting-review"
            aria-describedby="setting-review-hint"
            value={settings.reviewState}
            onChange={review}
            onBlur={onBlur}
          >
            {REVIEW_STATES.map((state) => (
              <option key={state} value={state}>
                {en.stage[state]}
              </option>
            ))}
          </select>
        </SettingField>
        {children}
        <MoreSettings settings={settings} onSetting={onSetting} onBlur={onBlur} />
      </div>
    </details>
  );
}
