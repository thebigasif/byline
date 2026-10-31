import { type FormEvent, useState } from "react";
import type { SiteSettings } from "../api/site";
import { en } from "../strings/en";
import { languageName } from "../strings/languages";
import { fieldsOf, languageCodes, type SiteFields } from "./siteChange";

type Props = { site: SiteSettings; onSave: (fields: SiteFields) => Promise<void> };

// The text fields, each with the input type a browser checks it by.
const TEXTS = [
  ["name", "text"],
  ["address", "url"],
  ["mount", "text"],
] as const;

/** The blog's name, where it lives, and its languages. */
export function SiteForm({ site, onSave }: Props) {
  const words = en.site.fields;
  const [fields, setFields] = useState(fieldsOf(site));
  const [busy, setBusy] = useState(false);
  const set = (key: keyof SiteFields, value: string) => setFields({ ...fields, [key]: value });
  const codes = languageCodes(fields.languages);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    await onSave(fields);
    setBusy(false);
  };

  return (
    <form className="max-w-160" onSubmit={submit}>
      <h2>{en.site.where}</h2>
      {TEXTS.map(([key, type]) => (
        <div className="field" key={key}>
          <label htmlFor={`site-${key}`}>{words[key].label}</label>
          <span className="hint" id={`site-${key}-hint`}>
            {words[key].hint}
          </span>
          <input
            id={`site-${key}`}
            type={type}
            required
            aria-describedby={`site-${key}-hint`}
            value={fields[key]}
            onChange={(event) => set(key, event.target.value)}
          />
        </div>
      ))}
      <p className="careful">{en.site.careful}</p>
      <h2>{en.site.languages}</h2>
      <div className="field">
        <label htmlFor="site-languages">{words.languages.label}</label>
        <span className="hint" id="site-languages-hint">
          {words.languages.hint}
        </span>
        <input
          id="site-languages"
          required
          aria-describedby="site-languages-hint"
          value={fields.languages}
          onChange={(event) => set("languages", event.target.value)}
        />
      </div>
      <div className="field">
        <label htmlFor="site-default">{words.default_language.label}</label>
        <span className="hint" id="site-default-hint">
          {words.default_language.hint}
        </span>
        <select
          id="site-default"
          aria-describedby="site-default-hint"
          value={fields.default_language}
          onChange={(event) => set("default_language", event.target.value)}
        >
          {codes.map((code) => (
            <option key={code} value={code}>
              {languageName(code)}
            </option>
          ))}
        </select>
      </div>
      <button type="submit" className="btn primary" disabled={busy}>
        {en.site.save}
      </button>
    </form>
  );
}
