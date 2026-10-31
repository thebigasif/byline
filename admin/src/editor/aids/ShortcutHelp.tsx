import { useEffect, useRef } from "react";
import { en } from "../../strings/en";

type Props = { onClose: () => void };

/** Every writing shortcut, what to type and what it gives. */
export function ShortcutHelp({ onClose }: Props) {
  const close = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    close.current?.focus();
  }, []);
  return (
    <section className="focus shortcut-help" aria-labelledby="shortcut-help-title">
      <h2 id="shortcut-help-title">{en.shortcuts.title}</h2>
      <p className="hint">{en.shortcuts.lede}</p>
      <table>
        <thead>
          <tr>
            <th scope="col">{en.shortcuts.typed}</th>
            <th scope="col">{en.shortcuts.gives}</th>
          </tr>
        </thead>
        <tbody>
          {en.shortcuts.rows.map(([typed, gives]) => (
            <tr key={typed}>
              <td>
                <kbd>{typed}</kbd>
              </td>
              <td>{gives}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <button ref={close} type="button" className="btn" onClick={onClose}>
        {en.shortcuts.close}
      </button>
    </section>
  );
}
