import type { ReactNode } from "react";

type Props = { id: string; label: string; hint: string; children: ReactNode };

/** One setting: its label, one line on what it does, then the field, which names `${id}-hint`. */
export function SettingField({ id, label, hint, children }: Props) {
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <span className="hint" id={`${id}-hint`}>{hint}</span>
      {children}
    </div>
  );
}
