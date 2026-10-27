import type { ChangeEvent } from "react";
import { en } from "../strings/en";
import { SettingField } from "./SettingField";
import type { SettingsDraft, TextSetting } from "./settings";

type More = keyof typeof en.settings.more.fields;
type Shape = { type?: "url" | "datetime-local"; rows?: number };

type Props = {
  settings: SettingsDraft;
  onSetting: (key: TextSetting, value: string) => void;
  onBlur: () => void;
};

const GROUPS: { legend: keyof typeof en.settings.more.groups; keys: More[] }[] = [
  { legend: "image", keys: ["imageSrc", "imageAlt", "imageCaption"] },
  { legend: "search", keys: ["canonical", "robots", "aliases"] },
  { legend: "x", keys: ["xTitle", "xDescription", "xImage"] },
  { legend: "facebook", keys: ["facebookTitle", "facebookDescription", "facebookImage"] },
  { legend: "down", keys: ["unpublishAt"] },
];

const SHAPES: Partial<Record<More, Shape>> = {
  canonical: { type: "url" },
  aliases: { rows: 3 },
  unpublishAt: { type: "datetime-local" },
  xDescription: { rows: 2 },
  facebookDescription: { rows: 2 },
};

/** The settings most posts leave alone, folded away: each says what it does, then the field. */
export function MoreSettings({ settings, onSetting, onBlur }: Props) {
  const words = en.settings.more;
  const field = (key: More) => {
    const id = `setting-${key}`;
    const shape = SHAPES[key] ?? {};
    const bind = {
      id,
      value: settings[key],
      "aria-describedby": `${id}-hint`,
      onChange: (event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) =>
        onSetting(key, event.target.value),
      onBlur,
    };
    let control = <input {...bind} type={shape.type ?? "text"} />;
    if (shape.rows !== undefined) {
      control = <textarea {...bind} rows={shape.rows} />;
    } else if (key === "robots") {
      const known = words.robots.some((option) => option.value === settings.robots);
      control = (
        <select {...bind}>
          {words.robots.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
          {known ? null : <option value={settings.robots}>{settings.robots}</option>}
        </select>
      );
    }
    return (
      <SettingField key={key} id={id} label={words.fields[key].label} hint={words.fields[key].hint}>
        {control}
      </SettingField>
    );
  };

  return (
    <details>
      <summary>{words.title}</summary>
      <div>
        {GROUPS.map((group) => (
          <fieldset key={group.legend}>
            <legend>{words.groups[group.legend]}</legend>
            {group.keys.map(field)}
          </fieldset>
        ))}
      </div>
    </details>
  );
}
